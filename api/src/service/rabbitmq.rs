use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRegistry,
		DeploymentRunningDetails,
		DeploymentStatus,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use lapin::{options::BasicPublishOptions, BasicProperties};

use crate::{
	db::Workspace,
	models::rabbitmq::{
		BYOCData,
		BillingData,
		CIData,
		DeploymentRequestData,
		InfraRequestData,
		Queue,
	},
	rabbitmq::{self, BuildId, BuildStep},
	service,
	utils::{settings::Settings, Error},
};

pub async fn queue_check_and_update_deployment_status(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::Deployment(
			DeploymentRequestData::CheckAndUpdateStatus {
				workspace_id: workspace_id.clone(),
				deployment_id: deployment_id.clone(),
			},
		),
		config,
		request_id,
	)
	.await
}

pub async fn queue_update_deployment_image(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	image_name: &str,
	digest: &str,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deployment_running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::Deployment(DeploymentRequestData::UpdateImage {
			workspace_id: workspace_id.clone(),
			deployment: Deployment {
				id: deployment_id.clone(),
				name: name.to_string(),
				registry: registry.clone(),
				image_tag: image_tag.to_string(),
				status: DeploymentStatus::Pushed,
				region: region.clone(),
				machine_type: machine_type.clone(),
				current_live_digest: Some(digest.to_string()),
			},
			image_name: image_name.to_owned(),
			digest: digest.to_string(),
			running_details: deployment_running_details.clone(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}

pub async fn queue_process_payment(
	month: u32,
	year: i32,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_billing_queue(
		&BillingData::ProcessWorkspaces {
			month,
			year,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn queue_attempt_to_charge_workspace(
	workspace: &Workspace,
	month: u32,
	year: i32,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_billing_queue(
		&BillingData::SendInvoiceForWorkspace {
			workspace: workspace.clone(),
			month,
			year,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn queue_retry_payment_for_workspace(
	workspace_id: &Uuid,
	process_after: &DateTime<Utc>,
	month: u32,
	year: i32,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	send_message_to_billing_queue(
		&BillingData::RetryPaymentForWorkspace {
			workspace_id: workspace_id.clone(),
			process_after: (*process_after).into(),
			month,
			year,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn queue_generate_invoice_for_workspace(
	config: &Settings,
	workspace: Workspace,
	month: u32,
	year: i32,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	send_message_to_billing_queue(
		&BillingData::GenerateInvoice {
			month,
			year,
			workspace,
			request_id: request_id.clone(),
		},
		config,
		&request_id,
	)
	.await
}

pub async fn send_message_to_infra_queue(
	message: &InfraRequestData,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			&Queue::Infrastructure.to_string(),
			BasicPublishOptions::default(),
			serde_json::to_string(&message)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	if !confirmation.is_ack() {
		log::error!("request_id: {} - RabbitMQ publish failed", request_id);
		return Err(Error::empty());
	}

	channel.close(200, "Normal shutdown").await.map_err(|e| {
		log::error!("Error closing rabbitmq channel: {}", e);
		Error::from(e)
	})?;

	rabbitmq_connection
		.close(200, "Normal shutdown")
		.await
		.map_err(|e| {
			log::error!("Error closing rabbitmq connection: {}", e);
			Error::from(e)
		})?;
	Ok(())
}

pub async fn send_message_to_ci_queue(
	message: &CIData,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			&Queue::Ci.to_string(),
			BasicPublishOptions::default(),
			serde_json::to_string(&message)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	if !confirmation.is_ack() {
		log::error!("request_id: {} - RabbitMQ publish failed", request_id);
		return Err(Error::empty());
	}

	channel.close(200, "Normal shutdown").await.map_err(|e| {
		log::error!("Error closing rabbitmq channel: {}", e);
		Error::from(e)
	})?;

	rabbitmq_connection
		.close(200, "Normal shutdown")
		.await
		.map_err(|e| {
			log::error!("Error closing rabbitmq connection: {}", e);
			Error::from(e)
		})?;
	Ok(())
}

pub async fn send_message_to_billing_queue(
	message: &BillingData,
	_config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let app = service::get_app();
	let (channel, rabbitmq_connection) =
		rabbitmq::get_rabbitmq_connection_channel(&app.rabbitmq).await?;

	let confirmation = channel
		.basic_publish(
			"",
			&Queue::Billing.to_string(),
			BasicPublishOptions::default(),
			serde_json::to_string(&message)?.as_bytes(),
			BasicProperties::default(),
		)
		.await?
		.await?;

	if !confirmation.is_ack() {
		log::error!("request_id: {} - RabbitMQ publish failed", request_id);
		return Err(Error::empty());
	}

	channel.close(200, "Normal shutdown").await.map_err(|e| {
		log::error!("Error closing rabbitmq channel: {}", e);
		Error::from(e)
	})?;

	rabbitmq_connection
		.close(200, "Normal shutdown")
		.await
		.map_err(|e| {
			log::error!("Error closing rabbitmq connection: {}", e);
			Error::from(e)
		})?;
	Ok(())
}

pub async fn queue_create_ci_build_step(
	build_step: BuildStep,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::BuildStep {
			build_step,
			request_id: request_id.clone(),
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_cancel_ci_build_pipeline(
	build_id: BuildId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::CancelBuild {
			build_id,
			request_id: request_id.clone(),
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_clean_ci_build_pipeline(
	build_id: BuildId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_ci_queue(
		&CIData::CleanBuild {
			build_id,
			request_id: request_id.clone(),
		},
		config,
		request_id,
	)
	.await
}

pub async fn queue_setup_kubernetes_cluster(
	region_id: &Uuid,
	cluster_url: &str,
	certificate_authority_data: &str,
	auth_username: &str,
	auth_token: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	send_message_to_infra_queue(
		&InfraRequestData::BYOC(BYOCData::InitKubernetesCluster {
			region_id: region_id.clone(),
			cluster_url: cluster_url.to_owned(),
			certificate_authority_data: certificate_authority_data.to_string(),
			auth_username: auth_username.to_string(),
			auth_token: auth_token.to_string(),
			request_id: request_id.clone(),
		}),
		config,
		request_id,
	)
	.await
}
