mod initializer;
mod meta_data;
mod migrations;
mod rbac;
mod user;
mod workspace;

use redis::{aio::MultiplexedConnection, Client, RedisError};
use sqlx::{pool::PoolOptions, Connection, Database as Db, Pool};
use tokio::task;

pub use self::{
	initializer::*,
	meta_data::*,
	migrations::*,
	rbac::*,
	user::*,
	workspace::*,
};
use crate::{query, utils::settings::Settings, Database};

pub async fn create_database_connection(
	config: &Settings,
) -> Result<Pool<Database>, sqlx::Error> {
	log::trace!("Creating database connection pool...");
	PoolOptions::<Database>::new()
		.max_connections(config.database.connection_limit)
		.connect_with(
			<<Database as Db>::Connection as Connection>::Options::new()
				.username(&config.database.user)
				.password(&config.database.password)
				.host(&config.database.host)
				.port(config.database.port)
				.database(&config.database.database),
		)
		.await
}

pub async fn create_redis_connection(
	config: &Settings,
) -> Result<MultiplexedConnection, RedisError> {
	log::trace!("Creating redis connection pool...");
	let (redis, redis_poller) = Client::open(format!(
		"{}://{}{}{}:{}/{}",
		if config.redis.secure {
			"rediss"
		} else {
			"redis"
		},
		if let Some(user) = &config.redis.user {
			user
		} else {
			""
		},
		if let Some(password) = &config.redis.password {
			format!(":{}@", password)
		} else {
			"".to_string()
		},
		config.redis.host,
		config.redis.port,
		if let Some(database) = config.redis.database {
			database
		} else {
			0
		}
	))?
	.create_multiplexed_tokio_connection()
	.await?;
	task::spawn(redis_poller);

	Ok(redis)
}

pub async fn begin_deferred_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn end_deferred_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
