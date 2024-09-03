use sqlx::{pool::PoolOptions, Pool};
use tokio::fs::{self, File};

use crate::prelude::*;

/// The initializer for the database. This will create the database pool and
/// initialize the database with the necessary tables and data.
pub(super) mod initializer;
/// The meta data for the database. This is mostly used for the version number
/// of the database and handling the migrations for the database.
pub(super) mod meta_data;
/// The workspace module for the database. This is used to handle the workspaces
/// and their data.
pub(super) mod workspace;

pub use self::initializer::initialize;
pub(super) use self::{meta_data::*, workspace::*};

/// Connects to the database based on a config. Not much to say here.
#[instrument(skip(config))]
pub async fn connect(config: &DatabaseConfig) -> Pool<DatabaseType> {
	info!("Connecting to database: `{}`", config.file);
	if fs::metadata(config.file.as_str()).await.is_err() {
		File::create(config.file.as_str())
			.await
			.expect("Failed to create database file");
	}
	PoolOptions::<DatabaseType>::new()
		.max_connections(config.connection_limit)
		.connect(&format!("sqlite://{}", config.file))
		.await
		.expect("Failed to connect to database")
}
