use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::infrastructure::{
		deployment::{
			CreateDeploymentRequest,
			CreateDeploymentResponse,
			DeleteDeploymentResponse,
			Deployment,
			DeploymentRegistry,
			DeploymentRunningDetails,
			DeploymentStatus,
			GetDeploymentInfoResponse,
			GetDeploymentLogsResponse,
			ListDeploymentsResponse,
			ListLinkedURLsResponse,
			PatrRegistry,
			StartDeploymentResponse,
			StopDeploymentResponse,
			UpdateDeploymentRequest,
			UpdateDeploymentResponse,
		},
		managed_urls::{ManagedUrl, ManagedUrlType},
	},
	utils::{constants, Uuid},
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use lapin::{options::BasicPublishOptions, BasicProperties};
use tokio::task;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		db_mapping::ManagedUrlType as DbManagedUrlType,
		rabbitmq::{DeploymentRequestData, RequestMessage},
		rbac::permissions,
	},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	// List all deployments
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_deployments)),
		],
	);

	// Create a new deployment
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::CREATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(create_deployment)),
		],
	);

	// Get info about a deployment
	app.get(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_deployment_info)),
		],
	);

	// start a deployment
	app.post(
		"/:deploymentId/start",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(start_deployment)),
		],
	);

	// stop the deployment
	app.post(
		"/:deploymentId/stop",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(stop_deployment)),
		],
	);

	// get logs for the deployment
	app.get(
		"/:deploymentId/logs",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_logs)),
		],
	);

	// Delete a deployment
	app.delete(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::DELETE,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(delete_deployment)),
		],
	);

	// Update a deployment
	app.patch(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_deployment)),
		],
	);

	// List all linked URLs of a deployment
	app.get(
		"/:deploymentId/managed-urls",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_linked_urls)),
		],
	);

	app
}

/// # Description
/// This function is used to list of all the deployments present with the user
/// required inputs:
/// workspaceId in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success:
///    deployments: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_deployments(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Listing deployments", request_id);

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Getting deployments from database",
		request_id
	);
	let deployments = db::get_deployments_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter_map(|deployment| {
		Some(Deployment {
			id: deployment.id,
			name: deployment.name,
			registry: if deployment.registry == constants::PATR_REGISTRY {
				DeploymentRegistry::PatrRegistry {
					registry: PatrRegistry,
					repository_id: deployment.repository_id?,
				}
			} else {
				DeploymentRegistry::ExternalRegistry {
					registry: deployment.registry,
					image_name: deployment.image_name?,
				}
			},
			image_tag: deployment.image_tag,
			status: deployment.status,
			region: deployment.region,
			machine_type: deployment.machine_type,
		})
	})
	.collect();
	log::trace!(
		"request_id: {} - Deployments successfully retreived",
		request_id
	);

	context.success(ListDeploymentsResponse { deployments });
	Ok(context)
}

/// # Description
/// This function is used to create a new deployment
/// required inputs
/// auth token in the header
/// workspace id in parameter
/// ```
/// {
///    name: ,
///    registry: ,
///    repositoryId: ,
///    imageName: ,
///    imageTag: ,
///    region: ,
///    domainName: ,
///    horizontalScale: ,
///    machineType:
/// }
/// ```
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success:
///    deploymentId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Creating deployment", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let CreateDeploymentRequest {
		workspace_id: _,
		name,
		registry,
		image_tag,
		region,
		machine_type,
		running_details:
			DeploymentRunningDetails {
				deploy_on_push,
				max_horizontal_scale,
				min_horizontal_scale,
				ports,
				environment_variables,
			},
		deploy_on_create,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.trim();
	let image_tag = image_tag.trim();

	let config = context.get_state().config.clone();

	log::trace!(
		"request_id: {} - Creating the deployment in workspace",
		request_id
	);
	let id = service::create_deployment_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		name,
		&registry,
		image_tag,
		&region,
		&machine_type,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		&ports
			.clone()
			.into_iter()
			.map(|(port, port_type)| (port.value(), port_type))
			.collect(),
		&environment_variables,
		&request_id,
	)
	.await?;

	// Check if image exists
	// If it does, push to docr.
	// Can't check for image existence for non-patr registry
	log::trace!("request_id: {} - Checking if image exists", request_id);
	if let DeploymentRegistry::PatrRegistry {
		registry: _,
		repository_id,
	} = &registry
	{
		log::trace!(
			"request_id: {} - Getting tag details from database",
			request_id
		);
		let tag_details = db::get_docker_repository_tag_details(
			context.get_database_connection(),
			repository_id,
			image_tag,
		)
		.await?;

		log::trace!(
			"request_id: {} - Getting repository details from the database",
			request_id
		);
		let repository_details = db::get_docker_repository_by_id(
			context.get_database_connection(),
			repository_id,
		)
		.await?
		.status(500)?;

		log::trace!(
			"request_id: {} - Getting workspace details from the database",
			request_id
		);
		let workspace_details = db::get_workspace_info(
			context.get_database_connection(),
			&repository_details.workspace_id,
		)
		.await?
		.status(500)?;

		log::trace!(
			"request_id: {} - Checking if the image exists",
			request_id
		);
		if let Some((_, digest)) = tag_details {
			log::trace!("request_id: {} - Image exists", request_id);
			// Push to docr
			let id = id.clone();
			let workspace_id = workspace_details.id.clone();
			let name = name.to_string();
			let image_tag = image_tag.to_string();
			let full_image = format!(
				"{}/{}/{}@{}",
				config.docker_registry.registry_url,
				workspace_details.name,
				repository_details.name,
				digest
			);
			let request_id = request_id.clone();

			db::update_deployment_status(
				context.get_database_connection(),
				&id,
				&DeploymentStatus::Pushed,
			)
			.await?;

			task::spawn(async move {
				let rabbitmq_result = service::get_rabbitmq_connection_channel(
					&config,
					&request_id,
				)
				.await;

				let (channel, rabbitmq_connection) =
					if let Ok((channel, rabbitmq_connection)) = rabbitmq_result
					{
						(channel, rabbitmq_connection)
					} else {
						log::error!("request_id: {} - Failed to get rabbitmq connection", request_id);
						return;
					};

				log::trace!(
					"request_id: {} - Acquiring database connection",
					request_id
				);
				let mut connection = if let Ok(connection) =
					service::get_app().database.acquire().await
				{
					connection
				} else {
					log::error!("request_id: {} - Unable to acquire a database connection", request_id);
					return;
				};
				log::trace!(
					"request_id: {} - Acquired database connection",
					request_id
				);

				let result = service::push_to_docr(
					&mut connection,
					&id,
					&full_image,
					&config,
					&request_id,
				)
				.await;

				if let Err(e) = result {
					log::error!(
						"Error pushing image to docr: {}",
						e.get_error()
					);
					return;
				}

				// If deploy_on_create is false, then return
				if !deploy_on_create {
					return;
				}

				let content =
					RequestMessage::Deployment(DeploymentRequestData::Update {
						workspace_id,
						deployment: Deployment {
							id,
							name,
							registry,
							image_tag,
							status: DeploymentStatus::Deploying,
							region,
							machine_type,
						},
						full_image,
						running_details: DeploymentRunningDetails {
							min_horizontal_scale,
							max_horizontal_scale,
							deploy_on_push,
							ports,
							environment_variables,
						},
						request_id: request_id.clone(),
					});

				let payload = serde_json::to_string(&content);

				let payload = if let Ok(payload) = payload {
					payload.into_bytes()
				} else {
					log::error!(
						"request_id: {} - Unable to serialize request message",
						request_id
					);
					return;
				};

				let publish_result = channel
					.basic_publish(
						"",
						"infrastructure",
						BasicPublishOptions::default(),
						&payload,
						BasicProperties::default(),
					)
					.await;

				let _result = match publish_result {
					Ok(publish_result) => match publish_result.await {
						Ok(result) => result,
						Err(e) => {
							log::error!("Error publishing message to infrastructure queue: {}", e);
							return;
						}
					},
					Err(e) => {
						log::error!(
							"Error publishing message to infrastructure queue: {}",
							e
						);
						return;
					}
				};

				if let Err(e) = channel.close(200, "Normal shutdown").await {
					log::error!("Error closing rabbitmq channel: {}", e);
				};

				if let Err(e) =
					rabbitmq_connection.close(200, "Normal shutdown").await
				{
					log::error!("Error closing rabbitmq connection: {}", e);
				};
			});
		}
	}

	let _ = service::get_deployment_metrics(
		context.get_database_connection(),
		"A new deployment has been created",
	)
	.await;

	context.success(CreateDeploymentResponse { id });
	Ok(context)
}

/// # Description
/// This function is used to get the information about a specific deployment
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// DeploymentId in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///     success: true or false,
///     deployment:
///     {
///         id: ,
///         name: ,
///         registry: ,
///         imageName: ,
///         imageTag: ,
///         domainName: ,
///         horizontalScale: ,
///         machineType:
///     }
/// }
/// ```
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_deployment_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"request_id: {} - Getting deployment details from the database for deployment: {}",
		request_id,
		deployment_id,
	);
	let (mut deployment, workspace_id, _, running_details) =
		service::get_full_deployment_config(
			context.get_database_connection(),
			&deployment_id,
			&request_id,
		)
		.await?;

	log::trace!("request_id: {} - Checking deployment status", request_id);
	deployment.status = match deployment.status {
		// If it's deploying or running, check with k8s on the actual status
		db_status @ (DeploymentStatus::Deploying |
		DeploymentStatus::Running) => {
			log::trace!(
				"request_id: {} - Deployment is deploying or running",
				request_id
			);
			let config = context.get_state().config.clone();
			let status = service::get_kubernetes_deployment_status(
				context.get_database_connection(),
				&deployment_id,
				workspace_id.as_str(),
				&config,
			)
			.await?;

			if db_status != status {
				log::trace!(
					"request_id: {} - Updating deployment status",
					request_id
				);
				db::update_deployment_status(
					context.get_database_connection(),
					&deployment_id,
					&status,
				)
				.await?;
			}
			status
		}
		status => status, // In all other cases, it is what it is
	};

	context.success(GetDeploymentInfoResponse {
		deployment,
		running_details,
	});
	Ok(context)
}

/// # Description
/// This function is used to start a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn start_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Start deployment", request_id);
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	// start the container running the image, if doesn't exist
	let config = context.get_state().config.clone();
	service::start_deployment(
		context.get_database_connection(),
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(StartDeploymentResponse {});
	Ok(context)
}

/// # Description
/// This function is used to stop a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn stop_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!("request_id: {} - Stopping deployment", request_id);
	// stop the running container, if it exists
	let config = context.get_state().config.clone();
	service::stop_deployment(
		context.get_database_connection(),
		&deployment_id,
		&config,
		&request_id,
		DeploymentStatus::Stopped,
	)
	.await?;

	context.success(StopDeploymentResponse {});
	Ok(context)
}

/// # Description
/// This function is used to get the logs of a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_logs(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();
	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Getting logs", request_id);
	// stop the running container, if it exists
	let logs = service::get_deployment_container_logs(
		context.get_database_connection(),
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(GetDeploymentLogsResponse { logs });
	Ok(context)
}

/// # Description
/// This function is used to delete deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!("request_id: {} - Deleting deployment", request_id);
	// stop and delete the container running the image, if it exists
	let config = context.get_state().config.clone();
	service::delete_deployment(
		context.get_database_connection(),
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	let _ = service::get_deployment_metrics(
		context.get_database_connection(),
		"A deployment has been deleted",
	)
	.await;

	context.success(DeleteDeploymentResponse {});
	Ok(context)
}

async fn update_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"{} - Updating deployment with id: {}",
		request_id,
		deployment_id
	);
	let UpdateDeploymentRequest {
		workspace_id: _,
		deployment_id: _,
		name,
		region,
		machine_type,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		ports,
		environment_variables,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.as_ref().map(|name| name.trim());

	// Is any one value present?
	if name.is_none() &&
		region.is_none() &&
		machine_type.is_none() &&
		deploy_on_push.is_none() &&
		min_horizontal_scale.is_none() &&
		max_horizontal_scale.is_none() &&
		ports.is_none() &&
		environment_variables.is_none()
	{
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let config = context.get_state().config.clone();

	service::update_deployment(
		context.get_database_connection(),
		&deployment_id,
		name,
		region.as_ref(),
		machine_type.as_ref(),
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		ports.as_ref(),
		environment_variables.as_ref(),
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	let (deployment, workspace_id, full_image, running_details) =
		service::get_full_deployment_config(
			context.get_database_connection(),
			&deployment_id,
			&request_id,
		)
		.await?;

	match &deployment.status {
		DeploymentStatus::Stopped |
		DeploymentStatus::Deleted |
		DeploymentStatus::Created => {
			// Don't update deployments that are explicitly stopped or deleted
		}
		_ => {
			db::update_deployment_status(
				context.get_database_connection(),
				&deployment_id,
				&DeploymentStatus::Deploying,
			)
			.await?;

			let (channel, connection) =
				service::get_rabbitmq_connection_channel(&config, &request_id)
					.await?;

			let content =
				RequestMessage::Deployment(DeploymentRequestData::Update {
					workspace_id,
					deployment,
					full_image,
					running_details,
					request_id: request_id.clone(),
				});

			let payload = serde_json::to_string(&content)?;

			let _publish_result = channel
				.basic_publish(
					"",
					"infrastructure",
					BasicPublishOptions::default(),
					payload.as_bytes(),
					BasicProperties::default(),
				)
				.await?
				.await?;
			channel.close(200, "Normal shutdown").await?;
			connection.close(200, "Normal shutdown").await?;
		}
	}

	context.success(UpdateDeploymentResponse {});
	Ok(context)
}

async fn list_linked_urls(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();
	let workspace_id = Uuid::parse_str(
		context.get_param(request_keys::WORKSPACE_ID).unwrap(),
	)?;

	let urls = db::get_all_managed_urls_for_deployment(
		context.get_database_connection(),
		&deployment_id,
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter_map(|url| {
		Some(ManagedUrl {
			id: url.id,
			sub_domain: url.sub_domain,
			domain_id: url.domain_id,
			path: url.path,
			url_type: match url.url_type {
				DbManagedUrlType::ProxyToDeployment => {
					ManagedUrlType::ProxyDeployment {
						deployment_id: url.deployment_id?,
						port: url.port? as u16,
					}
				}
				DbManagedUrlType::ProxyToStaticSite => {
					ManagedUrlType::ProxyStaticSite {
						static_site_id: url.static_site_id?,
					}
				}
				DbManagedUrlType::ProxyUrl => {
					ManagedUrlType::ProxyUrl { url: url.url? }
				}
				DbManagedUrlType::Redirect => {
					ManagedUrlType::Redirect { url: url.url? }
				}
			},
		})
	})
	.collect();

	context.success(ListLinkedURLsResponse { urls });
	Ok(context)
}
