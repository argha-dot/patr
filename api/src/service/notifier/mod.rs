use std::collections::HashMap;

use api_models::{models::auth::PreferredRecoveryOption, utils::Uuid};
use chrono::{Datelike, Month, Utc};
use eve_rs::AsError;
use num_traits::FromPrimitive;

use crate::{
	db::{self, User, UserToSignUp},
	error,
	models::deployment::KubernetesEventData,
	utils::Error,
	Database,
};

mod email;
mod sms;

/// # Description
/// This function is used to notify the user that their sign up has been
/// successfully completed. This will ideally introduce them to the platform and
/// give them details on what they can do
///
/// # Arguments
/// * `welcome_email` - an Option<String> containing either String which has
/// user's personal or business email to send a welcome notification to or
/// `None`
/// * `recovery_email` - an Option<String> containing either String which has
/// user's recovery email to send a verification email to or `None`
/// * `recovery_phone_number` - an Option<String> containing either String which
/// has user's recovery phone number to send a verification sms to or `None`
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_sign_up_complete_notification(
	welcome_email: Option<String>,
	recovery_email: Option<String>,
	recovery_phone_number: Option<String>,
	username: &str,
) -> Result<(), Error> {
	if let Some(welcome_email) = welcome_email {
		email::send_sign_up_completed_email(welcome_email.parse()?, username)
			.await?;
	}
	if let Some(recovery_email) = recovery_email {
		email::send_recovery_registration_mail(
			recovery_email.parse()?,
			username,
		)
		.await?;
	}
	if let Some(phone_number) = recovery_phone_number {
		sms::send_recovery_registration_sms(&phone_number).await?;
	}
	Ok(())
}

/// # Description
/// This function is used to send email to the user to verify
///
/// # Arguments
/// * `new_email` - an Option<String> containing either String which has
/// new email added by the user to which the otp will be sent or `None`
/// * `otp` - a string containing otp to be sent
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_email_verification_otp(
	new_email: String,
	otp: &str,
	username: &str,
) -> Result<(), Error> {
	email::send_email_verification_otp(new_email.parse()?, otp, username).await
}

/// # Description
/// This function is used to send otp to verify user's phone number
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `country_code` - a string containing 2 letter country code
/// * `phone_number` - a string containing phone number of user
/// * `otp` - a string containing otp to be sent for verification
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_phone_number_verification_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	sms::send_phone_number_verification_otp(
		connection,
		country_code,
		phone_number,
		otp,
	)
	.await
}

/// # Description
/// This function is used to send otp to the user for sign-up
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`UserToSignUp`]
/// * `otp` - a string containing otp to be sent
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_user_sign_up_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: &UserToSignUp,
	otp: &str,
) -> Result<(), Error> {
	// chcek if email is given as a recovery option
	if let Some((recovery_email_domain_id, recovery_email_local)) = user
		.recovery_email_domain_id
		.as_ref()
		.zip(user.recovery_email_local.as_ref())
	{
		let email = get_user_email(
			connection,
			recovery_email_domain_id,
			recovery_email_local,
		)
		.await?;
		email::send_user_verification_otp(email.parse()?, otp).await
	} else if let Some((phone_country_code, phone_number)) = user
		.recovery_phone_country_code
		.as_ref()
		.zip(user.recovery_phone_number.as_ref())
	{
		// check if phone number is given as a recovery
		let phone_number =
			get_user_phone_number(connection, phone_country_code, phone_number)
				.await?;
		sms::send_user_verification_otp(&phone_number, otp).await
	} else {
		Err(Error::empty()
			.status(400)
			.body(error!(NO_RECOVERY_OPTIONS).to_string()))
	}
}

/// # Description
/// This function is used to notify the user on all the recovery options that
/// their password has been changed
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_password_changed_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
) -> Result<(), Error> {
	// check if email is given as a recovery option
	if let Some((recovery_email_domain_id, recovery_email_local)) =
		user.recovery_email_domain_id.zip(user.recovery_email_local)
	{
		let email = get_user_email(
			connection,
			&recovery_email_domain_id,
			&recovery_email_local,
		)
		.await?;
		email::send_password_changed_notification(
			email.parse()?,
			&user.username,
		)
		.await?;
	}
	// check if phone number is given as a recovery
	if let Some((phone_country_code, phone_number)) = user
		.recovery_phone_country_code
		.zip(user.recovery_phone_number)
	{
		let phone_number = get_user_phone_number(
			connection,
			&phone_country_code,
			&phone_number,
		)
		.await?;
		sms::send_password_changed_notification(&phone_number).await?;
	}
	Ok(())
}

/// # Description
/// This function is used to notify the user that their password has been reset
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_user_reset_password_notification(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
) -> Result<(), Error> {
	if let Some((phone_country_code, phone_number)) = user
		.recovery_phone_country_code
		.zip(user.recovery_phone_number)
	{
		let phone_number = get_user_phone_number(
			connection,
			&phone_country_code,
			&phone_number,
		)
		.await?;
		sms::send_user_reset_password_notification(&phone_number).await?;
	}
	if let Some((recovery_email_domain_id, recovery_email_local)) =
		user.recovery_email_domain_id.zip(user.recovery_email_local)
	{
		let email = get_user_email(
			connection,
			&recovery_email_domain_id,
			&recovery_email_local,
		)
		.await?;
		email::send_user_reset_password_notification(
			email.parse()?,
			&user.username,
		)
		.await?;
	}
	Ok(())
}

/// # Description
/// This function is used to send otp incase the user forgets their password and
/// requests for a reset of their password
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user` - an object of type [`User`] containing all details of user
/// * `recovery_option` - an object of type [`PreferredRecoveryOption`]
/// * `otp` - a string containing otp to be sent to user
///  
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_forgot_password_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	user: User,
	recovery_option: &PreferredRecoveryOption,
	otp: &str,
) -> Result<(), Error> {
	// match on the recovery type
	match recovery_option {
		PreferredRecoveryOption::RecoveryEmail => {
			let email = get_user_email(
				connection,
				user.recovery_email_domain_id
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
				user.recovery_email_local
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
			)
			.await?;
			// send email
			email::send_forgot_password_otp(email.parse()?, otp, &user.username)
				.await
		}
		PreferredRecoveryOption::RecoveryPhoneNumber => {
			let phone_number = get_user_phone_number(
				connection,
				user.recovery_phone_country_code
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
				user.recovery_phone_number
					.as_ref()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
			)
			.await?;
			// send SMS
			sms::send_forgot_password_otp(&phone_number, otp).await
		}
	}
}

/// # Description
/// This function is used to get the user's email address, from a local email
/// and a domain ID
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `domain_id` - An unsigned 8 bit integer array containing id of
/// workspace domain
/// * `email_string` - a string containing user's email_local
///  
/// # Returns
/// This function returns `Result<String, Error>` containing user's email
/// address or an error
///
/// [`Transaction`]: Transaction
async fn get_user_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	email_string: &str,
) -> Result<String, Error> {
	let domain = db::get_personal_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;
	let email = format!("{}@{}", email_string, domain.name);
	Ok(email)
}

/// # Description
/// This function is used to get the user's complete phone number, with the
/// country code
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `country_code` - a string containing 2 letter country code
/// * `phone_number` - a string containing user's phone number
///  
/// # Returns
/// This function returns `Result<String, Error>` containing user's complete
/// phone number or an error
///
/// [`Transaction`]: Transaction
async fn get_user_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
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

/// # Description
/// This function is used to send alert to the user
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `workspace_name` - a Uuid containing id of the workspace
/// * `deployment_id` - a Uuid containing id of the deployment
/// * `deployment_name` - a string containing name of the deployment
/// * `message` - a string containing message of the alert
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_alert_email(
	workspace_name: &str,
	deployment_id: &Uuid,
	deployment_name: &str,
	message: &str,
	alert_emails: &[String],
) -> Result<(), Error> {
	// send email
	for email in alert_emails {
		email::send_alert_email(
			email.parse()?,
			workspace_name,
			deployment_id,
			deployment_name,
			message,
		)
		.await?;
	}

	Ok(())
}

/// # Description
/// This function is used to send alert to the patr's support email
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `workspace_name` - a Uuid containing id of the workspace
/// * `deployment_id` - a Uuid containing id of the deployment
/// * `deployment_name` - a string containing name of the deployment
/// * `event_data` - an object containing all the details of the event
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn send_alert_email_to_patr(
	event_data: KubernetesEventData,
) -> Result<(), Error> {
	// send email
	email::send_alert_email_to_patr("postmaster@vicara.co".parse()?, event_data)
		.await
}

pub async fn send_invoice_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	workspace_id: &Uuid,
	workspace_name: &str,
	deployment_usage: HashMap<Uuid, (u64, u64)>,
	static_site_usage_bill: u64,
	database_usage: HashMap<Uuid, (u64, u64)>,
	managed_url_usage_bill: u64,
	secret_usage_bill: u64,
	docker_repo_usage_bill: u64,
	domain_usage_bill: u64,
	total_bill: u64,
	start_date: &chrono::DateTime<Utc>,
	end_date: &chrono::DateTime<Utc>,
) -> Result<(), Error> {
	let user = db::get_user_by_user_id(connection, user_id)
		.await?
		.status(500)?;

	let user_email = get_user_email(
		connection,
		user.recovery_email_domain_id
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
		user.recovery_email_local
			.as_ref()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?,
	)
	.await?;

	let month = Month::from_u32(start_date.month())
		.status(500)?
		.name()
		.to_string();

	let year = start_date.year();

	let deployment_usage = deployment_usage
		.into_iter()
		.map(|(k, (bill, hours))| format!("{}-{}-{}", k, bill, hours))
		.collect::<Vec<String>>()
		.join("\n");

	let database_usage = database_usage
		.into_iter()
		.map(|(k, (bill, hours))| format!("{}-{}-{}", k, bill, hours))
		.collect::<Vec<String>>()
		.join("\n");

	email::send_invoice_email(
		user_email.parse()?,
		workspace_name.to_string(),
		deployment_usage,
		static_site_usage_bill,
		database_usage,
		managed_url_usage_bill,
		secret_usage_bill,
		docker_repo_usage_bill,
		domain_usage_bill,
		total_bill,
		month,
		year,
	)
	.await
}
