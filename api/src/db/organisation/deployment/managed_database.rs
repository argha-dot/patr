use api_macros::{query, query_as};

use crate::{
	models::db_mapping::{
		CloudPlatform,
		ManagedDatabase,
		ManagedDatabaseStatus,
	},
	Database,
};

pub async fn initialize_managed_database_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TYPE MANAGED_DATABASE_STATUS AS ENUM(
			'creating', /* Started the creation of database */
			'running', /* Database is running successfully */
			'stopped', /* Database is stopped by the user */
			'errored', /* Database encountered errors */
			'deleted' /* Database is deled by the user   */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CLOUD_PLATFORM AS ENUM(
			'aws',
			'digitalocean'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database(
			id BYTEA CONSTRAINT managed_database_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			cloud_database_id TEXT
				CONSTRAINT managed_database_uq_cloud_database_id UNIQUE,
			db_provider_name CLOUD_PLATFORM NOT NULL,
			engine TEXT,
			version TEXT,
			num_nodes INTEGER,
			size TEXT,
			region TEXT,
			status MANAGED_DATABASE_STATUS NOT NULL Default 'creating',
			host TEXT,
			port INTEGER,
			username TEXT,
			password TEXT, 
			organisation_id BYTEA NOT NULL,
			CONSTRAINT managed_database_uq_name_organisation_id UNIQUE (name, organisation_id),
			CONSTRAINT managed_database_chk_if_db_provdr_nme_and_db_id_is_valid CHECK(
				(
					cloud_database_id IS NOT NULL AND
					engine IS NOT NULL AND
					version IS NOT NULL AND
					num_nodes IS NOT NULL AND
					size IS NOT NULL AND
					region IS NOT NULL AND
					host IS NOT NULL AND
					port IS NOT NULL AND
					username IS NOT NULL AND
					password IS NOT NULL
				) OR
				(
					cloud_database_id IS NULL AND
					engine IS NULL AND
					version IS NULL AND
					num_nodes IS NULL AND
					size IS NULL AND
					region IS NULL AND
					host IS NULL AND
					port IS NULL AND
					username IS NULL AND
					password IS NULL
				)
			)
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
	log::info!("Finishing up managed_database tables initialization");
	query!(
		r#"
		ALTER TABLE managed_database 
			ADD CONSTRAINT managed_database_repository_fk_id_organisation_id
		FOREIGN KEY(id, organisation_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &[u8],
	name: &str,
	database_provider: CloudPlatform,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			managed_database
		VALUES
			($1, $2, NULL, $3, NULL, NULL, NULL, NULL, NULL, 'creating', NULL, NULL, NULL, NULL, $4);
		"#,
		id,
		name,
		database_provider as CloudPlatform,
		organisation_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_managed_database_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &[u8],
	status: &ManagedDatabaseStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			managed_database
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_database_clusters_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<ManagedDatabase>, sqlx::Error> {
	let rows = query_as!(
		ManagedDatabase,
		r#"
		SELECT
			id,
			name,
			cloud_database_id,
			db_provider_name as "db_provider_name: CloudPlatform",
			engine,
			version,
			num_nodes,
			size,
			region,
			status as "status: _",
			host,
			port,
			username,
			password,
			organisation_id
		FROM
			managed_database
		WHERE
			organisation_id = $1 AND
			cloud_database_id IS NOT NULL;
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn update_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	cloud_database_id: &str,
	engine: &str,
	version: &str,
	num_nodes: i32,
	size: &str,
	region: &str,
	status: ManagedDatabaseStatus,
	host: &str,
	port: i32,
	user: &str,
	password: &str,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE 
			managed_database
		SET
			cloud_database_id = $1,
			engine = $2,
			version = $3,
			num_nodes = $4,
			size = $5,
			region = $6,
			status = $7,
			host = $8,
			port = $9,
			username = $10,
			password = $11
		WHERE
			organisation_id = $12 AND
			name = $13;
		"#,
		cloud_database_id,
		engine,
		version,
		num_nodes,
		size,
		region,
		status as ManagedDatabaseStatus,
		host,
		port,
		user,
		password,
		organisation_id,
		name
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
