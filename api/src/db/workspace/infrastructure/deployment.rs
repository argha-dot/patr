use api_models::{
	models::workspace::infrastructure::deployment::{
		DeploymentStatus,
		ExposedPortType,
	},
	utils::Uuid,
};

use crate::{
	db,
	models::{
		db_mapping::{
			Deployment,
			DeploymentMachineType,
			DeploymentRegion,
			EnvVariable,
			WorkspaceAuditLog,
		},
		deployment::{
			DefaultDeploymentRegion,
			DEFAULT_DEPLOYMENT_REGIONS,
			DEFAULT_MACHINE_TYPES,
		},
	},
	query,
	query_as,
	Database,
};

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployments tables");

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_STATUS AS ENUM(
			'created', /* Created, but nothing pushed to it yet */
			'pushed', /* Something is pushed, but the system has not deployed it yet */
			'deploying', /* Something is pushed, and the system is currently deploying it */
			'running', /* Deployment is running successfully */
			'stopped', /* Deployment is stopped by the user */
			'errored', /* Deployment is stopped because of too many errors */
			'deleted' /* Deployment is deleted by the user */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_CLOUD_PROVIDER AS ENUM(
			'digitalocean'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO FIX REGION STORAGE
	query!(
		r#"
		CREATE TABLE deployment_region(
			id UUID CONSTRAINT deployment_region_pk PRIMARY KEY,
			name TEXT NOT NULL,
			provider DEPLOYMENT_CLOUD_PROVIDER,
			location GEOMETRY,
			parent_region_id UUID
				CONSTRAINT deployment_region_fk_parent_region_id
					REFERENCES deployment_region(id),
			CONSTRAINT
				deployment_region_chk_provider_location_parent_region_is_valid
				CHECK(
					(
						location IS NULL AND
						provider IS NULL
					) OR
					(
						provider IS NOT NULL AND
						location IS NOT NULL AND
						parent_region_id IS NOT NULL
					)
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_machine_type(
			id UUID CONSTRAINT deployment_machint_type_pk PRIMARY KEY,
			cpu_count SMALLINT NOT NULL,
			memory_count INTEGER NOT NULL /* Multiples of 0.25 GB */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment(
			id UUID CONSTRAINT deployment_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT deployment_chk_name_is_trimmed CHECK(
					name = TRIM(name)
				),
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.patr.cloud',
			repository_id UUID,
			image_name VARCHAR(512),
			image_tag VARCHAR(255) NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			workspace_id UUID NOT NULL,
			region UUID NOT NULL CONSTRAINT deployment_fk_region
				REFERENCES deployment_region(id),
			min_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_min_horizontal_scale_u8 CHECK(
					min_horizontal_scale >= 0 AND min_horizontal_scale <= 256
				)
				DEFAULT 1,
			max_horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_max_horizontal_scale_u8 CHECK(
					max_horizontal_scale >= 0 AND max_horizontal_scale <= 256
				)
				DEFAULT 1,
			machine_type UUID NOT NULL CONSTRAINT deployment_fk_machine_type
				REFERENCES deployment_machine_type(id),
			deploy_on_push BOOLEAN NOT NULL DEFAULT TRUE,
			CONSTRAINT deployment_fk_repository_id_workspace_id
				FOREIGN KEY(repository_id, workspace_id)
					REFERENCES docker_registry_repository(id, workspace_id),
			CONSTRAINT deployment_uq_name_workspace_id
				UNIQUE(name, workspace_id),
			CONSTRAINT deployment_uq_id_workspace_id
				UNIQUE(id, workspace_id),
			CONSTRAINT deployment_chk_repository_id_is_valid CHECK(
				(
					registry = 'registry.patr.cloud' AND
					image_name IS NULL AND
					repository_id IS NOT NULL
				)
				OR
				(
					registry != 'registry.patr.cloud' AND
					image_name IS NOT NULL AND
					repository_id IS NULL
				)
			),
			CONSTRAINT deployment_chk_image_tag_is_valid CHECK(
				image_tag != ''
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_name
		ON
			deployment
		(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_image_name_image_tag
		ON
			deployment
		(image_name, image_tag);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_registry_image_name_image_tag
		ON
			deployment
		(registry, image_name, image_tag);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id UUID
				CONSTRAINT deployment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(256) NOT NULL,
			value TEXT,
			secret_id UUID,
			CONSTRAINT deployment_environment_variable_pk
				PRIMARY KEY(deployment_id, name),
			CONSTRAINT deployment_environment_variable_fk_secret_id
				FOREIGN KEY(secret_id) REFERENCES secret(id),
			CONSTRAINT deployment_environment_variable_chk_value_secret_id_both_not_null
				CHECK(value IS NOT NULL OR secret_id IS NOT NULL)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE EXPOSED_PORT_TYPE AS ENUM(
			'http'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_exposed_port(
			deployment_id UUID
				CONSTRAINT deployment_exposed_port_fk_deployment_id
					REFERENCES deployment(id),
			port INTEGER NOT NULL CONSTRAINT
				deployment_exposed_port_chk_port_u16 CHECK(
					port > 0 AND port <= 65535
				),
			port_type EXPOSED_PORT_TYPE NOT NULL,
			CONSTRAINT deployment_exposed_port_pk
				PRIMARY KEY(deployment_id, port)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	for (cpu_count, memory_count) in DEFAULT_MACHINE_TYPES {
		let machine_type_id =
			db::generate_new_resource_id(&mut *connection).await?;
		query!(
			r#"
			INSERT INTO
				deployment_machine_type
			VALUES
				($1, $2, $3);
			"#,
			machine_type_id as _,
			cpu_count,
			memory_count
		)
		.execute(&mut *connection)
		.await?;
	}

	for continent in DEFAULT_DEPLOYMENT_REGIONS.iter() {
		let region_id =
			populate_region(&mut *connection, None, continent).await?;
		for country in continent.child_regions.iter() {
			let region_id =
				populate_region(&mut *connection, Some(&region_id), country)
					.await?;
			for city in country.child_regions.iter() {
				populate_region(&mut *connection, Some(&region_id), city)
					.await?;
			}
		}
	}

	Ok(())
}

pub async fn create_deployment_with_internal_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	name: &str,
	repository_id: &Uuid,
	image_tag: &str,
	workspace_id: &Uuid,
	region: &Uuid,
	machine_type: &Uuid,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			(
				$1,
				$2,
				'registry.patr.cloud',
				$3,
				NULL,
				$4,
				'created',
				$5,
				$6,
				$7,
				$8,
				$9,
				$10
			);
		"#,
		id as _,
		name as _,
		repository_id as _,
		image_tag,
		workspace_id as _,
		region as _,
		min_horizontal_scale as i32,
		max_horizontal_scale as i32,
		machine_type as _,
		deploy_on_push
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_deployment_with_external_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	name: &str,
	registry: &str,
	image_name: &str,
	image_tag: &str,
	workspace_id: &Uuid,
	region: &Uuid,
	machine_type: &Uuid,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			(
				$1,
				$2,
				$3,
				NULL,
				$4,
				$5,
				'created',
				$6,
				$7,
				$8,
				$9,
				$10,
				$11
			);
		"#,
		id as _,
		name as _,
		registry,
		image_name,
		image_tag,
		workspace_id as _,
		region as _,
		min_horizontal_scale as i32,
		max_horizontal_scale as i32,
		machine_type as _,
		deploy_on_push
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployments_by_image_name_and_tag_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	image_name: &str,
	image_tag: &str,
	workspace_id: &Uuid,
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			deployment.id as "id: _",
			deployment.name::TEXT as "name!: _",
			deployment.registry,
			deployment.repository_id as "repository_id: _",
			deployment.image_name,
			deployment.image_tag,
			deployment.status as "status: _",
			deployment.workspace_id as "workspace_id: _",
			deployment.region as "region: _",
			deployment.min_horizontal_scale,
			deployment.max_horizontal_scale,
			deployment.machine_type as "machine_type: _",
			deployment.deploy_on_push
		FROM
			deployment
		LEFT JOIN
			docker_registry_repository
		ON
			docker_registry_repository.id = deployment.repository_id
		WHERE
			(
				(
					deployment.registry = 'registry.patr.cloud' AND
					docker_registry_repository.name = $1
				) OR
				(
					deployment.registry != 'registry.patr.cloud' AND
					deployment.image_name = $1
				)
			) AND
			deployment.image_tag = $2 AND
			deployment.workspace_id = $3 AND
			deployment.status != 'deleted';
		"#,
		image_name as _,
		image_tag,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_deployments_by_repository_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push
		FROM
			deployment
		WHERE
			repository_id = $1 AND
			status != 'deleted';
		"#,
		repository_id as _
	)
	.fetch_all(&mut *connection)
	.await?;
	Ok(rows)
}

pub async fn get_deployments_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push
		FROM
			deployment
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Option<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_deployment_by_name_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	workspace_id: &Uuid,
) -> Result<Option<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			registry,
			repository_id as "repository_id: _",
			image_name,
			image_tag,
			status as "status: _",
			workspace_id as "workspace_id: _",
			region as "region: _",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type as "machine_type: _",
			deploy_on_push
		FROM
			deployment
		WHERE
			name = $1 AND
			workspace_id = $2 AND
			status != 'deleted';
		"#,
		name as _,
		workspace_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_deployment_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		deployment_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name as _,
		deployment_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<EnvVariable>, sqlx::Error> {
	query_as!(
		EnvVariable,
		r#"
		SELECT
		    deployment_id as "deployment_id: _",
			name,
			value,
			secret_id as "secret_id: _" 
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn add_environment_variable_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	key: &str,
	value: Option<&str>,
	secret_id: Option<&Uuid>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_environment_variable
		VALUES
			($1, $2, $3, $4);
		"#,
		deployment_id as _,
		key,
		value,
		secret_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_exposed_ports_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<(u16, ExposedPortType)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			port,
			port_type as "port_type: ExposedPortType"
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.port as u16, row.port_type))
	.collect();

	Ok(rows)
}

pub async fn add_exposed_port_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	port: u16,
	exposed_port_type: &ExposedPortType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_exposed_port
		VALUES
			($1, $2, $3);
		"#,
		deployment_id as _,
		port as i32,
		exposed_port_type as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_exposed_ports_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	name: Option<&str>,
	region: Option<&Uuid>,
	machine_type: Option<&Uuid>,
	deploy_on_push: Option<bool>,
	min_horizontal_scale: Option<u16>,
	max_horizontal_scale: Option<u16>,
) -> Result<(), sqlx::Error> {
	if let Some(name) = name {
		query!(
			r#"
			UPDATE
				deployment
			SET
				name = $1
			WHERE
				id = $2;
			"#,
			name as _,
			deployment_id as _
		)
		.execute(&mut *connection)
		.await?;
	}

	if let Some(region) = region {
		query!(
			r#"
			UPDATE
				deployment
			SET
				region = $1
			WHERE
				id = $2;
			"#,
			region as _,
			deployment_id as _
		)
		.execute(&mut *connection)
		.await?;
	}

	if let Some(machine_type) = machine_type {
		query!(
			r#"
			UPDATE
				deployment
			SET
				machine_type = $1
			WHERE
				id = $2;
			"#,
			machine_type as _,
			deployment_id as _
		)
		.execute(&mut *connection)
		.await?;
	}

	if let Some(deploy_on_push) = deploy_on_push {
		query!(
			r#"
			UPDATE
				deployment
			SET
				deploy_on_push = $1
			WHERE
				id = $2;
			"#,
			deploy_on_push as _,
			deployment_id as _
		)
		.execute(&mut *connection)
		.await?;
	}

	if let Some(min_horizontal_scale) = min_horizontal_scale {
		query!(
			r#"
			UPDATE
				deployment
			SET
				min_horizontal_scale = $1
			WHERE
				id = $2;
			"#,
			min_horizontal_scale as i32,
			deployment_id as _
		)
		.execute(&mut *connection)
		.await?;
	}

	if let Some(max_horizontal_scale) = max_horizontal_scale {
		query!(
			r#"
			UPDATE
				deployment
			SET
				max_horizontal_scale = $1
			WHERE
				id = $2;
			"#,
			max_horizontal_scale as i32,
			deployment_id as _
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

pub async fn get_all_deployment_machine_types(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<DeploymentMachineType>, sqlx::Error> {
	query_as!(
		DeploymentMachineType,
		r#"
		SELECT
			id as "id: _",
			cpu_count,
			memory_count
		FROM
			deployment_machine_type;
		"#
	)
	.fetch_all(&mut *connection)
	.await
}

async fn populate_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	parent_region_id: Option<&Uuid>,
	region: &DefaultDeploymentRegion,
) -> Result<Uuid, sqlx::Error> {
	let region_id = loop {
		let region_id = Uuid::new_v4();

		let row = query!(
			r#"
			SELECT
				id as "id: Uuid"
			FROM
				deployment_region
			WHERE
				id = $1;
			"#,
			region_id as _
		)
		.fetch_optional(&mut *connection)
		.await?;

		if row.is_none() {
			break region_id;
		}
	};

	if region.child_regions.is_empty() {
		// Populate leaf node
		query!(
			r#"
			INSERT INTO
				deployment_region
			VALUES
				($1, $2, $3, ST_SetSRID(POINT($4, $5)::GEOMETRY, 4326), $6);
			"#,
			region_id as _,
			region.name,
			region.cloud_provider.as_ref().unwrap() as _,
			region.coordinates.unwrap().0,
			region.coordinates.unwrap().1,
			parent_region_id as _,
		)
		.execute(&mut *connection)
		.await?;
	} else {
		// Populate parent node
		query!(
			r#"
			INSERT INTO
				deployment_region
			VALUES
				($1, $2, NULL, NULL, $3);
			"#,
			region_id as _,
			region.name,
			parent_region_id as _,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(region_id)
}

pub async fn get_all_deployment_regions(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<DeploymentRegion>, sqlx::Error> {
	query_as!(
		DeploymentRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _"
		FROM
			deployment_region;
		"#
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_build_events_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
) -> Result<Vec<WorkspaceAuditLog>, sqlx::Error> {
	query_as!(
		WorkspaceAuditLog,
		r#"
		SELECT
			workspace_audit_log.id as "id: _",
			date as "date: _",
			ip_address,
			workspace_id as "workspace_id: _",
			user_id as "user_id: _",
			login_id as "login_id: _",
			resource_id as "resource_id: _",
			permission.name as "action",
			request_id as "request_id: _",
			metadata as "metadata: _",
			patr_action as "patr_action: _",
			success
		FROM
			workspace_audit_log
		INNER JOIN
			permission
		ON
			permission.id = workspace_audit_log.action
		WHERE
			resource_id = $1 AND 
			(
				metadata ->> 'action' = 'create' OR
				metadata ->> 'action' = 'start' OR
				metadata ->> 'action' = 'stop' OR
				metadata ->> 'action' = 'updateImage'
			);
		"#,
		deployment_id as _
	)
	.fetch_all(&mut *connection)
	.await
}
