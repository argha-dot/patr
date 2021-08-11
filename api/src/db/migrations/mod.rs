use semver::Version;

mod from_v0;

/// This module is used to migrate the database to updated version
use crate::{db, utils::constants, Database};

/// # Description
/// The function is used to migrate the database from the current version to a
/// version set in ['Constants`]
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `current_db_version` - A struct containing version of the DB, more info
///   here: [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn migrate_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	current_db_version: Version,
) -> Result<(), sqlx::Error> {
	// Take a list of migrations available in the code.
	// Skip elements on the list until your current version is the same as the
	// migrating version
	// Then start migrating versions one by one until the end
	let mut migrations_from = get_migrations()
		.into_iter()
		.map(|version| {
			Version::parse(version).expect("unable to parse version")
		})
		.skip_while(|version| version != &current_db_version);

	while let Some(version) = migrations_from.next() {
		match (version.major, version.minor, version.patch) {
			(0, ..) => from_v0::migrate(&mut *connection, version).await?,
			_ => panic!(
				"Migration from version {} is not implemented yet!",
				version
			),
		}
	}

	db::set_database_version(connection, &constants::DATABASE_VERSION).await?;

	Ok(())
}

/// # Description
/// The function is used to get a list of all migrations to migrate the database
/// from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
const fn get_migrations() -> Vec<&'static str> {
	vec![
		from_v0::get_migrations(),
		// from_v0_4::get_migrations(),
	]
	.into_iter()
	.flatten()
	.collect()
}
