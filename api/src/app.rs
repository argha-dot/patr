use std::{
	fmt::{Debug, Formatter},
	process,
	sync::Arc,
	time::{Duration, Instant},
};

use colored::Colorize;
use eve_rs::{
	handlebars::Handlebars,
	listen,
	App as EveApp,
	AsError,
	Context,
	HttpMethod,
	NextHandler,
	Response,
};
use redis::aio::MultiplexedConnection as RedisConnection;
use sqlx::Pool;
use tokio::{signal, task, time};

use crate::{
	error,
	pin_fn,
	routes,
	service,
	utils::{settings::Settings, Error, ErrorData, EveContext, EveMiddleware},
	Database,
};

#[derive(Clone)]
pub struct App {
	pub config: Settings,
	pub database: Pool<Database>,
	pub redis: RedisConnection,
	pub render_register: Arc<Handlebars<'static>>,
}

impl Debug for App {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:#?}", self)
	}
}

pub async fn start_server(app: App) {
	let port = app.config.port;

	let mut eve_app = create_eve_app(&app);

	eve_app.set_error_handler(eve_error_handler);
	eve_app.use_middleware("/", get_basic_middlewares());
	eve_app.use_sub_app(&app.config.base_path, routes::create_sub_app(&app));

	log::info!("Listening for connections on 127.0.0.1:{}", port);
	let shutdown_signal = Some(get_shutdown_signal());
	let api_server_task =
		task::spawn(listen(eve_app, ([127, 0, 0, 1], port), shutdown_signal));
}

pub fn create_eve_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	EveApp::create(EveContext::new, app.clone())
}

#[cfg(debug_assertions)]
fn get_basic_middlewares() -> [EveMiddleware; 4] {
	[
		EveMiddleware::CustomFunction(pin_fn!(init_states)),
		EveMiddleware::CustomFunction(pin_fn!(add_cors_headers)),
		EveMiddleware::JsonParser,
		EveMiddleware::UrlEncodedParser,
	]
}

#[cfg(not(debug_assertions))]
fn get_basic_middlewares() -> [EveMiddleware; 6] {
	use eve_rs::default_middlewares::compression;
	[
		EveMiddleware::CustomFunction(pin_fn!(init_states)),
		EveMiddleware::CustomFunction(pin_fn!(add_cors_headers)),
		EveMiddleware::Compression(compression::DEFAULT_COMPRESSION_LEVEL),
		EveMiddleware::JsonParser,
		EveMiddleware::UrlEncodedParser,
		EveMiddleware::CookieParser,
	]
}

async fn get_shutdown_signal() {
	signal::ctrl_c()
		.await
		.expect("Unable to install signal handler");
	println!();
	log::warn!("Recieved stop signal. Gracefully shutting down server");
	tokio::spawn(async {
		time::sleep(Duration::from_millis(2000)).await;
		log::info!("Server taking too long to quit...");
		log::info!("Press Ctrl+C again to force quit application");
		signal::ctrl_c()
			.await
			.expect("Unable to install signal handler");
		println!();
		log::warn!("Recieved stop signal again. Force shutting down server");
		log::info!("Bye");
		process::exit(-1);
	});
}

fn eve_error_handler(mut response: Response, error: Error) -> Response {
	let error_string = error.get_error().to_string();
	if error_string != "entity not found" {
		log::error!(
			"Error occured while processing request: {}",
			error.get_error().to_string()
		);
	}
	response.set_content_type("application/json");
	response.set_status(error.get_status().unwrap_or(500));
	let default_error = error!(SERVER_ERROR).to_string();
	response.set_body_bytes(
		error
			.get_body_bytes()
			.unwrap_or_else(|| default_error.as_bytes()),
	);
	response
}

async fn init_states(
	mut context: EveContext,
	next: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// Start measuring time to check how long a route takes to execute
	let start_time = Instant::now();

	// Get a connection from the connection pool and begin a transaction on that
	// connection
	let transaction = context.get_state().database.begin().await?;

	// Set the database connection
	context.set_database_connection(transaction);
	let path = context.get_path();
	let method = context.get_method().clone();

	// Execute the next route and handle the result
	let result = next(context).await;

	// Log how long the request took, then either commit or rollback the
	// transaction
	let elapsed_time = start_time.elapsed();

	match result {
		Ok(mut context) => {
			context
				.take_database_connection()
				.commit()
				.await
				.body("Unable to commit transaction")?;
			log_request(
				&method,
				elapsed_time,
				&path,
				&context.get_status(),
				&context.get_response().get_body().len(),
			);

			Ok(context)
		}
		Err(err) => {
			log_request(
				&method,
				elapsed_time,
				&path,
				&err.get_status().unwrap_or(500),
				&err.get_body_bytes().unwrap_or(&[]).len(),
			);
			Err(err)
		}
	}
}

fn log_request(
	method: &HttpMethod,
	elapsed_time: Duration,
	path: &str,
	status: &u16,
	length: &usize,
) {
	log::info!(
		"{} {} {} {} - {}",
		method,
		path,
		match *status {
			100..=199 => format!("{}", status).normal(),
			200..=299 => format!("{}", status).green(),
			300..=399 => format!("{}", status).cyan(),
			400..=499 => format!("{}", status).yellow(),
			500..=599 => format!("{}", status).red(),
			_ => format!("{}", status).purple(),
		},
		if elapsed_time.as_millis() > 0 {
			format!("{} ms", elapsed_time.as_millis())
		} else {
			format!("{} μs", elapsed_time.as_micros())
		},
		length
	);
}

async fn add_cors_headers(
	mut context: EveContext,
	next: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	context
		.header("Access-Control-Allow-Origin", "*")
		.header("Access-Control-Allow-Methods", "*")
		.header("Access-Control-Allow-Headers", "Content-Type,Authorization");

	if context.get_method() == &HttpMethod::Options {
		return Ok(context);
	}

	next(context).await
}
