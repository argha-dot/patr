use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::{json, Value};

use crate::{
	app::{create_eve_app, App},
	db, error, pin_fn, service,
	utils::{
		constants::request_keys, Error, ErrorData, EveContext, EveMiddleware,
	},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	app.get(
		"/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_user_info)),
		],
	);
	app.post(
		"/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(update_user_info)),
		],
	);
	app.post(
		"/add-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(add_email_address)),
		],
	);
	app.get(
		"/list-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(list_email_addresses)),
		],
	);
	app.get(
		"/list-phone-numbers",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(list_phone_numbers)),
		],
	);
	app.post(
		"/update-backup-email",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(update_backup_email_address)),
		],
	);
	app.post(
		"/update-backup-phone",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(update_backup_phone_number)),
		],
	);
	app.post(
		"/add-phone-number",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(add_phone_number_for_user)),
		],
	);
	app.post(
		"/verify-phone-number",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(verify_phone_number)),
		],
	);
	app.delete(
		"/delete-personal-email",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(
				delete_personal_email_address
			)),
		],
	);
	app.delete(
		"/delete-phone-number",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(delete_phone_number)),
		],
	);
	app.post(
		"/verify-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(verify_email_address)),
		],
	);
	app.get(
		"/organisations",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_organisations_for_user)),
		],
	);
	app.post(
		"/change-password",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(change_password)),
		],
	);
	app.get(
		"/logins",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_all_logins_for_user)),
		],
	); // TODO list all logins here
	app.get(
		"/logins/:loginId/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_login_info)),
		],
	); // TODO list all information about a particular login here
	app.delete("/logins/:loginId", []); // TODO delete a particular login ID and invalidate it
	app.get(
		"/:username/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_user_info_by_username)),
		],
	);
	app
}

async fn get_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();
	let user =
		db::get_user_by_user_id(context.get_database_connection(), &user_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let personal_emails = db::get_personal_emails_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?;

	let phone_numbers = db::get_phone_numbers_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|phone_number| {
		json!({
			request_keys::COUNTRY_CODE: phone_number.country_code,
			request_keys::PHONE_NUMBER: phone_number.number
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::USERNAME: user.username,
		request_keys::FIRST_NAME: user.first_name,
		request_keys::LAST_NAME: user.last_name,
		request_keys::BIRTHDAY: user.dob,
		request_keys::BIO: user.bio,
		request_keys::LOCATION: user.location,
		request_keys::CREATED: user.created,
		request_keys::EMAILS: personal_emails,
		request_keys::PHONE_NUMBERS: phone_numbers
	}));
	Ok(context)
}

async fn get_user_info_by_username(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let username = context.get_param(request_keys::USERNAME).unwrap().clone();

	let user_data =
		db::get_user_by_username(context.get_database_connection(), &username)
			.await?
			.status(400)
			.body(error!(PROFILE_NOT_FOUND).to_string())?;

	let mut data = serde_json::to_value(user_data)?;
	let object = data.as_object_mut().unwrap();
	object.remove(request_keys::ID);
	object.insert(request_keys::SUCCESS.to_string(), true.into());

	context.json(json!(data));
	Ok(context)
}

async fn update_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let first_name = body
		.get(request_keys::FIRST_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let last_name = body
		.get(request_keys::LAST_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let dob = body
		.get(request_keys::BIRTHDAY)
		.map(|value| match value {
			Value::String(value) => value
				.parse::<u64>()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()),
			Value::Number(num) => {
				if let Some(num) = num.as_u64() {
					Ok(num)
				} else if let Some(num) = num.as_i64() {
					Ok(num as u64)
				} else {
					Err(Error::empty()
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string()))
				}
			}
			_ => Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())),
		})
		.transpose()?;

	let bio = body
		.get(request_keys::BIO)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let location = body
		.get(request_keys::LOCATION)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let dob_string = dob.map(|value| value.to_string());
	let dob_str = dob_string.as_deref();

	// If no parameters to update
	first_name
		.or(last_name)
		.or(dob_str)
		.or(bio)
		.or(location)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::update_user_data(
		context.get_database_connection(),
		first_name,
		last_name,
		dob,
		bio,
		location,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn add_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let email_address = body
		.get(request_keys::EMAIL)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::add_personal_email_to_be_verified_for_user(
		context.get_database_connection(),
		email_address,
		&user_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn list_email_addresses(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();

	let email_addresses_list = db::get_personal_emails_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::EMAILS: email_addresses_list
	}));
	Ok(context)
}

async fn list_phone_numbers(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();

	let phone_numbers_list = db::get_phone_numbers_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|phone_number| {
		json!({
			request_keys::COUNTRY_CODE: phone_number.country_code,
			request_keys::PHONE_NUMBER: phone_number.number
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::PHONE_NUMBERS: phone_numbers_list
	}));
	Ok(context)
}

async fn update_backup_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let email_address = body
		.get(request_keys::BACKUP_EMAIL)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	service::update_user_backup_email(
		context.get_database_connection(),
		&user_id,
		email_address,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn update_backup_phone_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let country_code = body
		.get(request_keys::BACKUP_PHONE_COUNTRY_CODE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let phone_number = body
		.get(request_keys::BACKUP_PHONE_NUMBER)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	service::update_user_backup_phone_number(
		context.get_database_connection(),
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn delete_personal_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let email_address = body
		.get(request_keys::EMAIL)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	service::delete_personal_email_address(
		context.get_database_connection(),
		&user_id,
		&email_address,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn add_phone_number_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();
	// two letter country code instead of the numeric one
	let country_code = body
		.get(request_keys::COUNTRY_CODE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let phone_number = body
		.get(request_keys::PHONE_NUMBER)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let otp = service::add_phone_number_to_be_verified_for_user(
		context.get_database_connection(),
		&user_id,
		country_code,
		phone_number,
	)
	.await?;

	service::send_phone_number_verification_otp(
		context.get_database_connection(),
		country_code,
		phone_number,
		&otp,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));

	Ok(context)
}

async fn verify_phone_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let country_code = body
		.get(request_keys::COUNTRY_CODE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let phone_number = body
		.get(request_keys::PHONE_NUMBER)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let otp = body
		.get(request_keys::VERIFICATION_TOKEN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::verify_phone_number_for_user(
		context.get_database_connection(),
		&user_id,
		country_code,
		phone_number,
		otp,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn delete_phone_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let country_code = body
		.get(request_keys::COUNTRY_CODE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let phone_number = body
		.get(request_keys::PHONE_NUMBER)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	service::delete_phone_number(
		context.get_database_connection(),
		&user_id,
		country_code,
		phone_number,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn verify_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let email_address = body
		.get(request_keys::EMAIL)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let otp = body
		.get(request_keys::VERIFICATION_TOKEN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::verify_personal_email_address_for_user(
		context.get_database_connection(),
		&user_id,
		email_address,
		otp,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn get_organisations_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();
	let organisations = db::get_all_organisations_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|org| {
		json!({
			request_keys::ID: org.id.encode_hex::<String>(),
			request_keys::NAME: org.name,
			request_keys::ACTIVE: org.active,
			request_keys::CREATED: org.created
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ORGANISATIONS: organisations
	}));
	Ok(context)
}

async fn change_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let new_password = body
		.get(request_keys::NEW_PASSWORD)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let password = body
		.get(request_keys::PASSWORD)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user =
		db::get_user_by_user_id(context.get_database_connection(), &user_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

	service::change_password_for_user(
		context.get_database_connection(),
		&user_id,
		password,
		new_password,
	)
	.await?;

	service::send_password_changed_notification(
		context.get_database_connection(),
		user,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn get_all_logins_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();

	let logins = db::get_all_logins_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|login| {
		let id = login.login_id.encode_hex::<String>();
		json!({
			request_keys::LOGIN_ID: id,
			request_keys::USER_ID: &user_id,
			request_keys::TOKEN_EXPIRY: login.token_expiry,
			request_keys::LAST_LOGIN: login.last_login,
			request_keys::LAST_ACTIVITY: login.last_activity
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::LOGINS: logins
	}));
	Ok(context)
}

async fn get_login_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let login_id = context
		.get_param(request_keys::LOGIN_ID)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.clone();

	let login_id = hex::decode(login_id)?;

	let login =
		db::get_user_login(context.get_database_connection(), &login_id)
			.await?
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::USER_ID: login.user_id,
		request_keys::LOGIN_ID: login.login_id,
		request_keys::TOKEN_EXPIRY: login.token_expiry,
		request_keys::LAST_LOGIN: login.last_login,
		request_keys::LAST_ACTIVITY: login.last_activity
	}));
	Ok(context)
}
