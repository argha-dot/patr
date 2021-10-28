use crate::{models::db_mapping::Workspace, query, query_as, Database};

mod docker_registry;
mod domain;
mod infrastructure;

pub use self::{docker_registry::*, domain::*, infrastructure::*};

pub async fn initialize_workspaces_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing workspace tables");
	query!(
		r#"
		CREATE TABLE workspace(
			id BYTEA
				CONSTRAINT workspace_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT workspace_uq_name UNIQUE,
			super_admin_id BYTEA NOT NULL
				CONSTRAINT workspace_super_admin_id_fk_user_id
					REFERENCES "user"(id),
			active BOOLEAN NOT NULL DEFAULT FALSE
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_idx_super_admin_id
		ON
			workspace
		(super_admin_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_idx_active
		ON
			workspace
		(active);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Ref: https://www.postgresql.org/docs/13/datatype-enum.html
	query!(
		r#"
		CREATE TYPE RESOURCE_OWNER_TYPE AS ENUM(
			'personal',
			'business'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	domain::initialize_domain_pre(connection).await?;
	docker_registry::initialize_docker_registry_pre(connection).await?;
	infrastructure::initialize_deployment_pre(connection).await?;

	Ok(())
}

pub async fn initialize_workspaces_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up workspace tables initialization");
	query!(
		r#"
		ALTER TABLE workspace
		ADD CONSTRAINT workspace_fk_id
		FOREIGN KEY(id) REFERENCES resource(id)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	domain::initialize_domain_post(connection).await?;
	docker_registry::initialize_docker_registry_post(connection).await?;
	infrastructure::initialize_deployment_post(connection).await?;

	Ok(())
}

pub async fn create_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
	name: &str,
	super_admin_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			workspace
		VALUES
			($1, $2, $3, $4);
		"#,
		workspace_id,
		name as _,
		super_admin_id,
		true,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_workspace_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
) -> Result<Option<Workspace>, sqlx::Error> {
	query_as!(
		Workspace,
		r#"
		SELECT
			id,
			name as "name: _",
			super_admin_id,
			active
		FROM
			workspace
		WHERE
			id = $1;
		"#,
		workspace_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_workspace_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
) -> Result<Option<Workspace>, sqlx::Error> {
	query_as!(
		Workspace,
		r#"
		SELECT
			id,
			name as "name: _",
			super_admin_id,
			active
		FROM
			workspace
		WHERE
			name = $1;
		"#,
		name as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_workspace_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			name = $1
		WHERE
			id = $2;
		"#,
		name as _,
		workspace_id,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
