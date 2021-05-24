use std::cmp::Ordering;

use crate::{
	app::App,
	db::{self, get_database_version, set_database_version},
	models::rbac,
	query,
	utils::constants,
};

pub async fn initialize(app: &App) -> Result<(), sqlx::Error> {
	log::info!("Initializing database");

	let tables = query!(
		r#"
		SELECT
			*
		FROM
			information_schema.tables
		WHERE
			table_catalog = $1 AND
			table_schema = 'public' AND
			table_type = 'BASE TABLE';
		"#,
		app.config.database.database
	)
	.fetch_all(&app.database)
	.await?;
	let mut transaction = app.database.begin().await?;

	// If no tables exist in the database, initialize fresh
	if tables.is_empty() {
		log::warn!("No tables exist. Creating fresh");

		// Create all tables
		db::initialize_meta_pre(&mut transaction).await?;
		db::initialize_users_pre(&mut transaction).await?;
		db::initialize_organisations_pre(&mut transaction).await?;
		db::initialize_rbac_pre(&mut transaction).await?;

		db::initialize_rbac_post(&mut transaction).await?;
		db::initialize_organisations_post(&mut transaction).await?;
		db::initialize_users_post(&mut transaction).await?;
		db::initialize_meta_post(&mut transaction).await?;

		// Set the database schema version
		set_database_version(&mut transaction, &constants::DATABASE_VERSION)
			.await?;

		transaction.commit().await?;

		log::info!("Database created fresh");
		Ok(())
	} else {
		// If it already exists, perform a migration with the known values

		let version = get_database_version(app).await?;

		match version.cmp(&constants::DATABASE_VERSION) {
			Ordering::Greater => {
				log::error!("Database version is higher than what's recognised. Exiting...");
				panic!();
			}
			Ordering::Less => {
				log::info!(
					"Migrating from {}.{}.{}",
					version.major,
					version.minor,
					version.patch
				);

				db::migrate_database(&mut transaction, version).await?;
			}
			Ordering::Equal => {
				log::info!("Database already in the latest version. No migration required.");
			}
		}

		// Initialize data
		// If a god UUID already exists, set it

		let god_uuid = db::get_god_user_id(&mut transaction).await?;
		if let Some(uuid) = god_uuid {
			rbac::GOD_USER_ID
				.set(uuid)
				.expect("GOD_USER_ID was already set");
		}

		let resource_types =
			db::get_all_resource_types(&mut transaction).await?;
		let resource_types = resource_types
			.into_iter()
			.map(|resource_type| (resource_type.name, resource_type.id))
			.collect();
		rbac::RESOURCE_TYPES
			.set(resource_types)
			.expect("RESOURCE_TYPES is already set");

		drop(transaction);

		Ok(())
	}
}
