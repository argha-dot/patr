use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	managed_url_redirects(&mut *connection, config).await?;
	Ok(())
}

pub(super) async fn managed_url_redirects(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_url
		ADD COLUMN permanent_redirect BOOLEAN,
		ADD COLUMN http_only BOOLEAN;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
