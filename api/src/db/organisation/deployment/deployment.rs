use crate::{
	models::db_mapping::{Deployment, DeploymentMachineType, DeploymentStatus},
	query,
	query_as,
	Database,
};

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
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
		CREATE TYPE DEPLOYMENT_MACHINE_TYPE AS ENUM(
			'micro',
			'small',
			'medium',
			'large'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment(
			id BYTEA CONSTRAINT deployment_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.patr.cloud',
			repository_id BYTEA CONSTRAINT deployment_fk_repository_id
				REFERENCES docker_registry_repository(id),
			image_name VARCHAR(512),
			image_tag VARCHAR(255) NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			deployed_image TEXT,
			digital_ocean_app_id TEXT
				CONSTRAINT deployment_uq_digital_ocean_app_id UNIQUE,
			region TEXT NOT NULL DEFAULT 'do-blr',
			horizontal_scale SMALLINT NOT NULL
				CONSTRAINT deployment_chk_horizontal_scale_u8 CHECK(
					horizontal_scale >= 0 AND horizontal_scale <= 256
				)
				DEFAULT 1,
			machine_type DEPLOYMENT_MACHINE_TYPE NOT NULL DEFAULT 'small',
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
			deployment_id BYTEA
				CONSTRAINT deploymment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(256) NOT NULL,
			value TEXT NOT NULL,
			CONSTRAINT deployment_environment_variable_pk
				PRIMARY KEY(deployment_id, name)
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
		ALTER TABLE deployment ADD CONSTRAINT deployment_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_deployment_with_internal_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	name: &str,
	repository_id: &[u8],
	image_tag: &str,
	region: &str,
	horizontal_scale: u64,
	machine_type: &DeploymentMachineType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			($1, $2, 'registry.patr.cloud', $3, NULL, $4, 'created', NULL, NULL, $5, $6, $7);
		"#,
		deployment_id,
		name,
		repository_id,
		image_tag,
		region,
		horizontal_scale as i16,
		machine_type as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_deployment_with_external_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	name: &str,
	registry: &str,
	image_name: &str,
	image_tag: &str,
	region: &str,
	horizontal_scale: u64,
	machine_type: &DeploymentMachineType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			($1, $2, $3, NULL, $4, $5, 'created', NULL, NULL, $6, $7, $8);
		"#,
		deployment_id,
		name,
		registry,
		image_name,
		image_tag,
		region,
		horizontal_scale as i16,
		machine_type as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployments_by_image_name_and_tag_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	image_name: &str,
	image_tag: &str,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			deployment.id,
			deployment.name,
			deployment.registry,
			deployment.repository_id,
			deployment.image_name,
			deployment.image_tag,
			deployment.status as "status: _",
			deployment.deployed_image,
			deployment.digital_ocean_app_id,
			deployment.region,
			deployment.horizontal_scale,
			deployment.machine_type as "machine_type: _"
		FROM
			deployment
		INNER JOIN
            resource
		ON
			deployment.id = resource.id
		LEFT JOIN
			docker_registry_repository
		ON
			docker_registry_repository.id = deployment.repository_id
		WHERE
			(
				(
					registry = 'registry.patr.cloud' AND
					docker_registry_repository.name = $1
				) OR
				(
					registry != 'registry.patr.cloud' AND
					image_name = $1
				)
			) AND
			image_tag = $2 AND
			resource.owner_id = $3;
		"#,
		image_name,
		image_tag,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn get_deployments_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			deployment.id,
			deployment.name,
			deployment.registry,
			deployment.repository_id,
			deployment.image_name,
			deployment.image_tag,
			deployment.status as "status: _",
			deployment.deployed_image,
			deployment.digital_ocean_app_id,
			deployment.region,
			deployment.horizontal_scale,
			deployment.machine_type as "machine_type: _"
		FROM
			deployment
		INNER JOIN
			resource
		ON
			deployment.id = resource.id
		WHERE
			resource.id = deployment.id AND
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn get_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Option<Deployment>, sqlx::Error> {
	let row = query_as!(
		Deployment,
		r#"
		SELECT
			id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status as "status: _",
			deployed_image,
			digital_ocean_app_id,
			deployment.region,
			deployment.horizontal_scale,
			deployment.machine_type as "machine_type: _"
		FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next();

	Ok(row)
}

pub async fn delete_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_deployed_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	deployed_image: Option<&str>,
) -> Result<(), sqlx::Error> {
	if let Some(deployed_image) = deployed_image {
		query!(
			r#"
			UPDATE
				deployment
			SET
				deployed_image = $1
			WHERE
				id = $2;
			"#,
			deployed_image,
			deployment_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			UPDATE
				deployment
			SET
				deployed_image = NULL
			WHERE
				id = $1;
			"#,
			deployment_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn update_digital_ocean_app_id_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	app_deployment_id: &str,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			digital_ocean_app_id = $1
		WHERE
			id = $2;
		"#,
		app_deployment_id,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
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
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Vec<(String, String)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.name, row.value))
	.collect();
	Ok(rows)
}

pub async fn add_environment_variable_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	key: &str,
	value: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_environment_variable
		VALUES
			($1, $2, $3);
		"#,
		deployment_id,
		key,
		value
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_horizontal_scale_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	horizontal_scale: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			horizontal_scale = $1
		WHERE
			id = $2;
		"#,
		horizontal_scale as i16,
		deployment_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_machine_type_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	machine_type: &DeploymentMachineType,
) -> Result<(), sqlx::Error> {
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
		deployment_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
