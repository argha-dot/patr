use eve_rs::AsError;
use sqlx::Transaction;

use crate::{
	db,
	error,
	models::db_mapping::{PreferredRecoveryOption, User, UserToSignUp},
	utils::Error,
	Database,
};

mod email;
mod sms;

pub use email::*;
pub use sms::*;

pub async fn send_sign_up_complete_notification(
	welcome_email: Option<String>,
	backup_email: Option<String>,
	backup_phone_number: Option<String>,
) -> Result<(), Error> {
	if let Some(welcome_email) = welcome_email {
		email::send_sign_up_completed_email(&welcome_email)?;
	}

	if let Some(backup_email) = backup_email {
		email::send_backup_registration_mail(&backup_email)?;
	}

	if let Some(phone_number) = backup_phone_number {
		sms::send_backup_registration_sms(&phone_number)?;
	}
	Ok(())
}
// could possibly also take in `PreferredNotifierType`
pub async fn send_user_sign_up_otp(
	connection: &mut Transaction<'_, Database>,
	user: UserToSignUp,
	otp: &str,
) -> Result<(), Error> {
	log::error!("NOTIFIER NOT YET IMPLEMENTED, Thanks for trying");
	// chcek if email is given as a backup option
	if let Some((_backup_email_domain_id, _backup_email_local)) = user
		.backup_email_domain_id
		.as_ref()
		.zip(user.backup_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			user.backup_email_domain_id.unwrap().as_ref(),
			&user.backup_email_local.unwrap(),
		)
		.await?;

		// this will panic
		email::send_user_verification_otp(&email, otp)?;
	}

	// check if phone number is given as a backup
	if let Some((_phone_country_code, _phone_number)) = user
		.backup_phone_country_code
		.as_ref()
		.zip(user.backup_phone_number.as_ref())
	{
		let phone_number = get_user_phone_number(
			connection,
			&user.backup_phone_country_code.unwrap(),
			&user.backup_phone_number.unwrap(),
		)
		.await?;

		// this will panic
		sms::send_user_verification_otp(&phone_number, otp)?;
	}

	Ok(())
}

// This function will send the given otp to all the backup options available for
// the given user.
pub async fn send_password_changed_notification(
	connection: &mut Transaction<'_, Database>,
	user: User,
) -> Result<(), Error> {
	// chcek if email is given as a backup option
	if let Some((_backup_email_domain_id, _backup_email_local)) = user
		.backup_email_domain_id
		.as_ref()
		.zip(user.backup_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			user.backup_email_domain_id.unwrap().as_ref(),
			&user.backup_email_local.unwrap(),
		)
		.await?;

		// this will panic
		email::send_password_changed_notification(&email)?;
	}

	// check if phone number is given as a backup
	if let Some((_phone_country_code, _phone_number)) = user
		.backup_phone_country_code
		.as_ref()
		.zip(user.backup_phone_number.as_ref())
	{
		let phone_number = get_user_phone_number(
			connection,
			&user.backup_phone_country_code.unwrap(),
			&user.backup_phone_number.unwrap(),
		)
		.await?;

		// this will panic
		sms::send_password_changed_notification(&phone_number)?;
	}
	Ok(())
}

// reset password
pub async fn send_user_reset_password_notification(
	connection: &mut Transaction<'_, Database>,
	user: User,
) -> Result<(), Error> {
	log::error!("NOTIFIER NOT YET IMPLEMENTED, Thanks for trying LOL");

	if let Some((_phone_country_code, _phone_number)) = user
		.backup_phone_country_code
		.as_ref()
		.zip(user.backup_phone_number.as_ref())
	{
		let phone_number = get_user_phone_number(
			connection,
			&user.backup_phone_country_code.unwrap(),
			&user.backup_phone_number.unwrap(),
		)
		.await?;

		// this will panic
		sms::send_user_reset_password_notification(&phone_number)?;
	}

	if let Some((_backup_email_domain_id, _backup_email_local)) = user
		.backup_email_domain_id
		.as_ref()
		.zip(user.backup_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			user.backup_email_domain_id.unwrap().as_ref(),
			&user.backup_email_local.unwrap(),
		)
		.await?;

		// this will panic
		email::send_user_reset_password_notification(&email)?;
	}
	Ok(())
}

pub async fn send_forgot_password_otp(
	connection: &mut Transaction<'_, Database>,
	user: User,
	recovery_option: PreferredRecoveryOption,
	otp: &str,
) -> Result<(), Error> {
	// match on the recovery type
	match recovery_option {
		PreferredRecoveryOption::BackupEmail => {
			let domain = db::get_personal_domain_by_id(
				connection,
				user.backup_email_domain_id.unwrap().as_ref(),
			)
			.await?
			.status(500)?;

			let email =
				format!("{}@{}", user.backup_email_local.unwrap(), domain.name);
			// send email
			email::send_user_verification_otp(&email, otp)?;
		}
		PreferredRecoveryOption::BackupPhoneNumber => {
			let phone_number = user.backup_phone_number.unwrap();
			let country_code = db::get_phone_country_by_country_code(
				connection,
				&user.backup_phone_country_code.unwrap(),
			)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

			let phone_number =
				format!("+{}{}", country_code.phone_code, phone_number);

			// send SMS
			sms::send_user_verification_otp(&phone_number, otp)?;
		}
		_ => Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?,
	};

	Ok(())
}

async fn get_user_email(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
	email_string: &str,
) -> Result<String, Error> {
	let domain = db::get_personal_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;
	let email = format!("{}@{}", email_string, domain.name);
	Ok(email)
}

async fn get_user_phone_number(
	connection: &mut Transaction<'_, Database>,
	country_code: &str,
	phone_number: &str,
) -> Result<String, Error> {
	let country_code =
		db::get_phone_country_by_country_code(connection, country_code)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

	let phone_number = format!("+{}{}", country_code.phone_code, phone_number);
	Ok(phone_number)
}
