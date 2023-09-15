use crate::prelude::*;

mod sign_up;
mod user_data;
mod user_email;
mod user_login;
mod user_phone;

pub use self::{
	sign_up::*,
	user_data::*,
	user_email::*,
	user_login::*,
	user_phone::*,
};

/// Initializes all user tables
#[instrument(skip(connection))]
pub async fn initialize_users_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Initializing user tables");
	user_data::initialize_user_data_tables(&mut *connection).await?;
	user_email::initialize_user_email_tables(&mut *connection).await?;
	user_phone::initialize_user_phone_tables(&mut *connection).await?;
	user_login::initialize_user_login_tables(&mut *connection).await?;
	sign_up::initialize_user_sign_up_tables(&mut *connection).await?;

	Ok(())
}

/// Finishes up all user tables
#[instrument(skip(connection))]
pub async fn initialize_users_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Finishing up user tables initialization");
	user_data::initialize_user_data_constraints(&mut *connection).await?;
	user_email::initialize_user_email_constraints(&mut *connection).await?;
	user_phone::initialize_user_phone_constraints(&mut *connection).await?;
	user_login::initialize_user_login_constraints(&mut *connection).await?;
	sign_up::initialize_user_sign_up_constraints(&mut *connection).await?;

	Ok(())
}
