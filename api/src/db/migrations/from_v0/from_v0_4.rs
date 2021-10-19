use semver::Version;

use crate::{migrate_query as query, Database};

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.x.x, more info here: [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	version: Version,
) -> Result<(), sqlx::Error> {
	match (version.major, version.minor, version.patch) {
		(0, 4, 0) => migrate_from_v0_4_0(&mut *connection).await?,
		(0, 4, 1) => migrate_from_v0_4_1(&mut *connection).await?,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}

	Ok(())
}

/// # Description
/// The function is used to get a list of all 0.3.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec!["0.4.0", "0.4.1"]
}

async fn migrate_from_v0_4_0(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_1(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_database
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
