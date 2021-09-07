use std::{ops::DerefMut, time::Duration};

use eve_rs::AsError;
use lightsail::model::RelationalDatabase;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use serde_json::json;
use tokio::{
	task,
	time::{self, Instant},
};

use crate::{
	db,
	error,
	models::{
		db_mapping::{
			CloudPlatform,
			DatabasePlan,
			Engine,
			ManagedDatabaseStatus,
		},
		deployment::cloud_providers::digitalocean::{
			DatabaseConfig,
			DatabaseInfo,
			DatabaseResponse,
			DbConnection,
		},
		rbac,
	},
	service::{self, deployment::aws},
	utils::{
		get_current_time,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
	Database,
};

pub async fn create_database_cluster(
	settings: Settings,
	name: &str,
	db_name: &str,
	version: Option<&str>,
	engine: &str,
	num_nodes: Option<u64>,
	region: &str,
	organisation_id: &[u8],
	database_plan: DatabasePlan,
) -> Result<(), Error> {
	let name = name.to_string();
	let db_name = db_name.to_string();
	let version = version.map(|v| v.to_string());
	let engine = engine.to_string();
	let organisation_id = organisation_id.to_vec();
	let (provider, region) = region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let region = region.to_string();

	match provider.parse() {
		Ok(CloudPlatform::DigitalOcean) => {
			task::spawn(async move {
				let result = create_database_on_digitalocean(
					settings,
					name,
					&db_name,
					version,
					engine,
					num_nodes,
					region,
					organisation_id,
					database_plan,
				)
				.await;

				if let Err(error) = result {
					log::error!(
						"Error while creating database, {}",
						error.get_error()
					);
				}
			});
		}
		Ok(CloudPlatform::Aws) => {
			task::spawn(async move {
				let result = create_database_on_aws(
					name,
					db_name,
					version,
					engine,
					region,
					organisation_id,
					database_plan,
				)
				.await;

				if let Err(error) = result {
					log::error!(
						"Error while creating database, {}",
						error.get_error()
					);
				}
			});
		}
		_ => {
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	}

	Ok(())
}

async fn create_database_on_digitalocean(
	settings: Settings,
	name: String,
	db_name: &str,
	version: Option<String>,
	engine: String,
	num_nodes: Option<u64>,
	region: String,
	organisation_id: Vec<u8>,
	database_plan: DatabasePlan,
) -> Result<(), Error> {
	log::trace!("creating a digital ocean managed database");
	let app = service::get_app();
	let engine = engine.parse::<Engine>()?;

	log::trace!("checking if the database name is valid or not");
	if !validator::is_database_name_valid(&name) {
		log::trace!("database name invalid");
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let version = if engine == Engine::Postgres {
		version.unwrap_or_else(|| "12".to_string())
	} else {
		version.unwrap_or_else(|| "8".to_string())
	};

	let client = Client::new();

	let num_nodes = num_nodes.unwrap_or(1);

	log::trace!("generating new resource");
	let resource_id =
		db::generate_new_resource_id(app.database.acquire().await?.deref_mut())
			.await?;
	let resource_id = resource_id.as_bytes();

	let db_resource_name =
		format!("database-{}", get_current_time().as_millis());

	let db_engine = if engine == Engine::Postgres {
		"pg"
	} else {
		"mysql"
	};

	let region = match region.as_str() {
		"nyc" => "nyc1",
		"ams" => "ams3",
		"sfo" => "sfo3",
		"sgp" => "sgp1",
		"lon" => "lon1",
		"fra" => "fra1",
		"tor" => "tor1",
		"blr" => "blr1",
		_ => "blr1",
	};

	log::trace!("sending the create db cluster request to digital ocean");
	let database_cluster = client
		.post("https://api.digitalocean.com/v2/databases")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&DatabaseConfig {
			name: db_resource_name, // should be unique
			engine: db_engine.to_string(),
			version: Some(version.clone()),
			num_nodes,
			size: database_plan.to_string(),
			region: region.to_string(),
		})
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;
	log::trace!("database created");

	db::create_resource(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&format!("do-database-{}", hex::encode(resource_id)),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		&organisation_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("resource generation complete");

	let region = format!("do-{}", region);

	log::trace!("creating entry for newly created managed database");
	db::create_managed_database(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&name,
		db_name,
		engine,
		&version,
		num_nodes as i32,
		&database_plan.to_string(),
		&region,
		&database_cluster.database.connection.host,
		database_cluster.database.connection.port as i32,
		&database_cluster.database.connection.user,
		&database_cluster.database.connection.password,
		&organisation_id,
		Some(&database_cluster.database.id),
	)
	.await?;

	log::trace!("updating to the db status to creating");
	// wait for database to start
	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&ManagedDatabaseStatus::Creating,
	)
	.await?;

	log::trace!("waiting for database to be online");
	wait_for_digitalocean_database_cluster_to_be_online(
		app.database.acquire().await?.deref_mut(),
		&settings,
		resource_id,
		&database_cluster.database.id,
		&client,
	)
	.await?;
	log::trace!("database online");

	log::trace!("creating a new database inside cluster");
	let new_db_status = client
		.post(format!(
			"https://api.digitalocean.com/v2/databases/{}/dbs",
			database_cluster.database.id
		))
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&json!({ "name": db_name }))
		.send()
		.await?
		.status();

	if new_db_status.is_client_error() || new_db_status.is_server_error() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	log::trace!("updating to the db status to running");
	// wait for database to start
	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&ManagedDatabaseStatus::Running,
	)
	.await?;
	log::trace!("database successfully updated");

	Ok(())
}

async fn create_database_on_aws(
	name: String,
	db_name: String,
	version: Option<String>,
	engine: String,
	region: String,
	organisation_id: Vec<u8>,
	database_plan: DatabasePlan,
) -> Result<(), Error> {
	log::trace!("creating a aws managed database");
	let app = service::get_app();
	let engine = engine.parse::<Engine>()?;

	log::trace!("checking if the database name is valid or not");
	if !validator::is_database_name_valid(&name) {
		log::trace!("database name invalid");
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let version = if engine == Engine::Postgres {
		version.unwrap_or_else(|| "12".to_string())
	} else {
		version.unwrap_or_else(|| "8".to_string())
	};
	let master_username = format!("user_{}", name);
	let client = aws::get_lightsail_client(&region);

	log::trace!("generating new resource");
	let resource_id =
		db::generate_new_resource_id(app.database.acquire().await?.deref_mut())
			.await?;
	let resource_id = resource_id.as_bytes();

	db::create_resource(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&format!("aws-database-{}", hex::encode(resource_id)),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_DATABASE)
			.unwrap(),
		&organisation_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("resource generation complete");

	let password: String = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect();

	log::trace!("sending the create db cluster request to aws");
	client
		.create_relational_database()
		.master_database_name(&name)
		.master_username(&master_username)
		.master_user_password(&password)
		.publicly_accessible(true)
		.relational_database_blueprint_id(format!("{}_{}", engine, version))
		.relational_database_bundle_id(database_plan.to_string())
		.relational_database_name(hex::encode(&resource_id))
		.send()
		.await?;
	log::trace!("database created");

	log::trace!("waiting for database to be online");
	let database_info = wait_for_aws_database_cluster_to_be_online(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		client,
	)
	.await?;
	log::trace!("database online");

	let address = database_info
		.master_endpoint
		.clone()
		.map(|rde| rde.address)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let port = database_info
		.master_endpoint
		.map(|port| port.port)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	log::trace!("creating entry for newly created managed database");

	let region = format!("aws-{}", region);

	db::create_managed_database(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&name,
		&db_name,
		engine,
		&version,
		1,
		&database_plan.to_string(),
		&region,
		&address,
		port,
		&master_username,
		&password,
		&organisation_id,
		None,
	)
	.await?;

	log::trace!("updating to the db status to running");
	// wait for database to start
	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		resource_id,
		&ManagedDatabaseStatus::Running,
	)
	.await?;
	log::trace!("database successfully updated");

	Ok(())
}

pub async fn get_all_database_clusters_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: Settings,
	organisation_id: &[u8],
) -> Result<Vec<DatabaseResponse>, Error> {
	let clusters = db::get_all_running_database_clusters_for_organisation(
		connection,
		organisation_id,
	)
	.await?;

	log::trace!("get a list of database clusters");

	let mut cluster_list = Vec::new();
	let client = Client::new();
	for cluster in clusters {
		let (provider, _) = cluster
			.region
			.split_once('-')
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
		let provider = provider.parse::<CloudPlatform>()?;
		match provider {
			CloudPlatform::Aws => {
				log::trace!("getting databases from aws");
				let managed_db_cluster = get_cluster_from_aws(
					&hex::encode(&cluster.id),
					&cluster.region,
					&cluster.password,
				)
				.await?;
				cluster_list.push(managed_db_cluster);
			}
			CloudPlatform::DigitalOcean => {
				log::trace!("getting databases from digitalocean");
				if let Some(digital_ocean_db_id) = cluster.digital_ocean_db_id {
					let managed_db_cluster = get_cluster_from_digital_ocean(
						&client,
						&settings,
						&digital_ocean_db_id,
						&cluster.id,
					)
					.await?;
					cluster_list.push(managed_db_cluster);
				}
			}
		}
	}

	Ok(cluster_list)
}

async fn wait_for_digitalocean_database_cluster_to_be_online(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: &Settings,
	database_id: &[u8],
	digital_ocean_db_id: &str,
	client: &Client,
) -> Result<(), Error> {
	let start = Instant::now();
	loop {
		let database_status = get_cluster_from_digital_ocean(
			client,
			settings,
			digital_ocean_db_id,
			database_id,
		)
		.await?;

		if database_status.database.status == *"online" {
			db::update_managed_database_status(
				connection,
				database_id,
				&ManagedDatabaseStatus::Running,
			)
			.await?;
			break;
		}

		if start.elapsed() > Duration::from_secs(900) {
			db::update_managed_database_status(
				connection,
				database_id,
				&ManagedDatabaseStatus::Errored,
			)
			.await?;
			let settings = settings.clone();
			let database_id = database_id.to_vec();
			let cloud_db_id = digital_ocean_db_id.to_string();
			let client = client.clone();
			task::spawn(async move {
				let result = wait_and_delete_the_running_database(
					&settings,
					&database_id,
					&cloud_db_id,
					&client,
				)
				.await;
				if let Err(error) = result {
					log::info!(
						"Error while creating databse: {}",
						error.get_error()
					);
				}
			});
			return Error::as_result()
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
	Ok(())
}

async fn wait_for_aws_database_cluster_to_be_online(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &[u8],
	client: lightsail::Client,
) -> Result<RelationalDatabase, Error> {
	loop {
		let database_info = client
			.get_relational_database()
			.relational_database_name(hex::encode(resource_id))
			.send()
			.await?;

		let database_state = database_info
			.clone()
			.relational_database
			.map(|rdbms| rdbms.state)
			.flatten()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		if database_state == "available" {
			db::update_managed_database_status(
				connection,
				resource_id,
				&ManagedDatabaseStatus::Running,
			)
			.await?;

			let db_info = database_info
				.relational_database
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;

			return Ok(db_info);
		} else if database_state != "creating" &&
			database_state != "configuring-log-exports" &&
			database_state != "backing-up"
		{
			break;
		}
		time::sleep(Duration::from_millis(1000)).await;
	}

	db::update_managed_database_status(
		connection,
		resource_id,
		&ManagedDatabaseStatus::Errored,
	)
	.await?;

	Error::as_result()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
}

async fn wait_and_delete_the_running_database(
	settings: &Settings,
	database_id: &[u8],
	cloud_db_id: &str,
	client: &Client,
) -> Result<(), Error> {
	let app = service::get_app();
	log::trace!("retreiving database: {} from digitalocean", cloud_db_id);
	loop {
		let database_status = get_cluster_from_digital_ocean(
			client,
			settings,
			cloud_db_id,
			database_id,
		)
		.await?;

		if database_status.database.status == *"online" {
			delete_managed_database(
				app.database.acquire().await?.deref_mut(),
				settings,
				database_id,
				client,
			)
			.await?;
			break;
		}
		time::sleep(Duration::from_millis(1000)).await;
	}
	log::trace!("database retreived");
	Ok(())
}

async fn get_cluster_from_digital_ocean(
	client: &Client,
	settings: &Settings,
	digital_ocean_db_id: &str,
	resource_id: &[u8],
) -> Result<DatabaseResponse, Error> {
	let mut database_status = client
		.get(format!(
			"https://api.digitalocean.com/v2/databases/{}",
			digital_ocean_db_id
		))
		.bearer_auth(&settings.digital_ocean_api_key)
		.send()
		.await?
		.json::<DatabaseResponse>()
		.await?;

	database_status.database.id = hex::encode(resource_id);
	Ok(database_status)
}

async fn get_cluster_from_aws(
	cloud_db_id: &str,
	region: &str,
	password: &str,
) -> Result<DatabaseResponse, Error> {
	let client = aws::get_lightsail_client(region);

	log::trace!("retrieving database: {} from aws", cloud_db_id);
	let database_cluster = client
		.get_relational_database()
		.relational_database_name(cloud_db_id)
		.send()
		.await?;
	log::trace!("database retreived from aws");

	log::trace!("getting id");
	let id = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.name)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("getting name");
	let name = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.master_database_name)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("getting engine");
	let engine = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.engine)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.parse::<Engine>()?
		.to_string();

	log::trace!("getting version");
	let version = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.engine_version)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("getting size of instance");
	let size = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.relational_database_bundle_id)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.parse::<DatabasePlan>()?
		.to_string();

	log::trace!("getting region");
	let region = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.location)
		.flatten()
		.map(|region| region.availability_zone)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let region = &region[0..region.len() - 1];

	log::trace!("getting status");
	let status = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.state)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.parse::<ManagedDatabaseStatus>()?
		.to_string();

	log::trace!("getting time of creation");
	let created_at = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.created_at)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.epoch_seconds()
		.to_string();

	log::trace!("getting host url");
	let host = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.master_endpoint)
		.flatten()
		.map(|m_endpt| m_endpt.address)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("getting user id");
	let user = database_cluster
		.relational_database
		.clone()
		.map(|rdb| rdb.master_username)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("getting port");
	let port = database_cluster
		.relational_database
		.map(|rdb| rdb.master_endpoint)
		.flatten()
		.map(|m_endpt| m_endpt.port)
		.flatten()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let connection = DbConnection {
		host,
		user,
		password: password.to_string(),
		port: port as u64,
	};

	let database_response = DatabaseResponse {
		database: DatabaseInfo {
			id,
			name,
			engine,
			version,
			num_nodes: 1,
			size,
			region: region.to_string(),
			status,
			created_at,
			connection,
			users: None,
		},
	};
	log::trace!("database retreived");
	Ok(database_response)
}

pub async fn get_managed_database_info_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: &Settings,
	resource_id: &[u8],
) -> Result<(DatabaseInfo, ManagedDatabaseStatus), Error> {
	let cluster = db::get_managed_database_by_id(connection, resource_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if let ManagedDatabaseStatus::Errored = cluster.status {
		return Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	} else if let ManagedDatabaseStatus::Deleted = cluster.status {
		return Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let client = Client::new();

	let (provider, _) = cluster
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let provider = provider.parse::<CloudPlatform>()?;
	let database_info = match provider {
		CloudPlatform::Aws => {
			log::trace!("getting cluster from aws");
			get_cluster_from_aws(
				&hex::encode(cluster.id),
				&cluster.region,
				&cluster.password,
			)
			.await?
		}
		CloudPlatform::DigitalOcean => {
			log::trace!("getting cluster from digital ocean");
			if let Some(digital_ocean_db_id) = cluster.digital_ocean_db_id {
				get_cluster_from_digital_ocean(
					&client,
					settings,
					&digital_ocean_db_id,
					&cluster.id,
				)
				.await?
			} else {
				return Err(Error::empty()
					.status(500)
					.body(error!(SERVER_ERROR).to_string()));
			}
		}
	};

	Ok((database_info.database, cluster.status))
}

pub async fn delete_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	settings: &Settings,
	resource_id: &[u8],
	client: &Client,
) -> Result<(), Error> {
	let cluster = db::get_managed_database_by_id(connection, resource_id)
		.await?
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let (provider, region) = cluster
		.region
		.split_once('-')
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let provider = provider.parse::<CloudPlatform>()?;

	match provider {
		CloudPlatform::Aws => {
			delete_managed_database_on_aws(&hex::encode(&cluster.id), region)
				.await?;
		}
		CloudPlatform::DigitalOcean => {
			if let Some(digital_ocean_db_id) = cluster.digital_ocean_db_id {
				delete_managed_database_on_digitalocean(
					&digital_ocean_db_id,
					settings,
					client,
				)
				.await?;
			}
		}
	}

	db::update_managed_database_status(
		connection,
		&cluster.id,
		&ManagedDatabaseStatus::Deleted,
	)
	.await?;

	Ok(())
}

async fn delete_managed_database_on_digitalocean(
	digital_ocean_db_id: &str,
	settings: &Settings,
	client: &Client,
) -> Result<(), Error> {
	let database_status = client
		.delete(format!(
			"https://api.digitalocean.com/v2/databases/{}",
			digital_ocean_db_id
		))
		.bearer_auth(&settings.digital_ocean_api_key)
		.send()
		.await?
		.status();

	if database_status.is_client_error() || database_status.is_server_error() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}
	Ok(())
}

async fn delete_managed_database_on_aws(
	cloud_db_id: &str,
	region: &str,
) -> Result<(), Error> {
	let client = aws::get_lightsail_client(region);

	let database_cluster = client
		.get_relational_database()
		.relational_database_name(cloud_db_id)
		.send()
		.await;

	if database_cluster.is_err() {
		return Ok(());
	}

	client
		.delete_relational_database()
		.relational_database_name(cloud_db_id)
		.send()
		.await?;

	Ok(())
}
