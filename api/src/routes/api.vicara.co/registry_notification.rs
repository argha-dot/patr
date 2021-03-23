use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use validator::is_docker_repo_name_valid;

use crate::{
	app::{create_eve_app, App},
	db, error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{constants::request_keys, validator, EveContext, EveMiddleware},
};
use serde_json::{json, Value};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/notification",
		&[EveMiddleware::CustomFunction(pin_fn!(notification_handler))],
	);
	sub_app
}

pub async fn notification_handler(
	context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}
