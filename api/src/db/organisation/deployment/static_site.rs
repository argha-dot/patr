use crate::{
	models::db_mapping::{DeploymentStaticSite, DeploymentStatus},
	query,
	query_as,
	Database,
};

pub async fn initialize_static_sites_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing static sites tables");

	query!(
		r#"
		CREATE TABLE deployment_static_sites(
			id BYTEA CONSTRAINT deployment_static_sites_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			status DEPLOYMENT_STATUS NOT NULL DEFAULT 'created',
			domain_name VARCHAR(255)
				CONSTRAINT deployment_static_sites_pk_uq_domain_name UNIQUE
				CONSTRAINT
					deployment_static_sites_pk_chk_domain_name_is_lower_case
						CHECK(domain_name = LOWER(domain_name)),
			organisation_id BYTEA NOT NULL,
			CONSTRAINT deployment_static_sites_uq_name_organisation_id
				UNIQUE(name, organisation_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_static_sites_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up static sites tables initialization");

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_fk_id_organisation_id
		FOREIGN KEY(id, organisation_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	name: &str,
	domain_name: Option<&str>,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	if let Some(domain) = domain_name {
		query!(
			r#"
			INSERT INTO
				deployment_static_sites
			VALUES
				($1, $2, 'created', $3, $4);
			"#,
			static_site_id,
			name,
			domain,
			organisation_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			INSERT INTO
				deployment_static_sites
			VALUES
				($1, $2, 'created', NULL, $3);
			"#,
			static_site_id,
			name,
			organisation_id
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}

pub async fn get_static_site_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
) -> Result<Option<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id,
			name,
			status as "status: _",
			domain_name,
			organisation_id
		FROM
			deployment_static_sites
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		static_site_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_static_site_deployment_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
) -> Result<Option<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id,
			name,
			status as "status: _",
			domain_name,
			organisation_id
		FROM
			deployment_static_sites
		WHERE
			name = $1 AND
			status != 'deleted';
		"#,
		name
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_static_site_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	status: &DeploymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		status as _,
		static_site_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_static_sites_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<DeploymentStaticSite>, sqlx::Error> {
	query_as!(
		DeploymentStaticSite,
		r#"
		SELECT
			id,
			name,
			status as "status: _",
			domain_name,
			organisation_id
		FROM
			deployment_static_sites
		WHERE
			organisation_id = $1 AND
			status != 'deleted';
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn set_domain_name_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	static_site_id: &[u8],
	domain_name: Option<&str>,
) -> Result<(), sqlx::Error> {
	if let Some(domain_name) = domain_name {
		query!(
			r#"
			UPDATE
				deployment_static_sites
			SET
				domain_name = $1
			WHERE
				id = $2;
			"#,
			domain_name,
			static_site_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	} else {
		query!(
			r#"
			UPDATE
				deployment_static_sites
			SET
				domain_name = NULL
			WHERE
				id = $1;
			"#,
			static_site_id,
		)
		.execute(&mut *connection)
		.await
		.map(|_| ())
	}
}
