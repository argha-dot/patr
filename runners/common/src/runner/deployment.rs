use std::{pin::pin, str::FromStr};

use futures::StreamExt;
use models::api::workspace::deployment::*;
use tokio::time::{Duration, Instant};

use crate::{prelude::*, utils::delayed_future::DelayedFuture};

impl<E> super::Runner<E>
where
	E: RunnerExecutor + Clone + 'static,
{
	/// Reconcile all the deployments that the runner is responsible for. This
	/// function will run the reconciliation for all the deployments that the
	/// runner is responsible for.
	pub(super) async fn reconcile_all_deployments(&mut self) {
		// Reconcile all deployments
		info!("Reconciling all deployments");

		// Update running deployments
		let Ok(mut should_run_deployments) = self.get_all_local_deployments().await else {
			return;
		};

		let mut running_deployments = pin!(self.executor.list_running_deployments().await);

		while let Some(deployment_id) = running_deployments.next().await {
			let deployment = should_run_deployments
				.iter()
				.find(|&&id| deployment_id == id);

			// If the deployment does not exist in the should run list, delete it
			let Some(&deployment_id) = deployment else {
				trace!(
					"Deployment `{}` does not exist in the should run list",
					deployment_id
				);
				info!("Deleting deployment `{}`", deployment_id);

				if let Err(wait_time) = self.executor.delete_deployment(deployment_id).await {
					self.reconciliation_list.push(DelayedFuture::new(
						Instant::now() + wait_time,
						deployment_id,
					));
					self.recheck_next_reconcile_future();
				}
				return;
			};

			// If it does exist, reconcile the deployment and remove it from the should run
			// list
			self.reconcile_deployment(deployment_id).await;
			should_run_deployments.retain(|&id| id != deployment_id);
		}

		// All remaining deployments are the ones that are there in the should run list,
		// but aren't running. So get them up and running
		for deployment_id in should_run_deployments {
			self.reconcile_deployment(deployment_id).await;
		}
	}

	/// Reconcile a specific deployment. This function will run the
	/// reconciliation for a specific deployment (based on the ID)
	pub(super) async fn reconcile_deployment(&mut self, deployment_id: Uuid) {
		trace!("Reconciling deployment `{}`", deployment_id);
		self.reconciliation_list
			.retain(|message| message.value() != &deployment_id);

		let result = 'reconcile: {
			let Ok(GetDeploymentInfoResponse {
				deployment,
				running_details,
			}) = self.get_deployment_info(deployment_id).await
			else {
				break 'reconcile Err(Duration::from_secs(5));
			};

			if let Err(err) = self
				.executor
				.upsert_deployment(deployment, running_details)
				.await
			{
				break 'reconcile Err(err);
			}

			Ok(())
		};

		if let Err(wait_time) = result {
			self.reconciliation_list.push(DelayedFuture::new(
				Instant::now() + wait_time,
				deployment_id,
			));
		}

		self.recheck_next_reconcile_future();
	}

	/// Get all the local deployments. This function will get all the local
	/// deployments from the SQLite database.
	async fn get_all_local_deployments(&mut self) -> Result<Vec<Uuid>, ErrorType> {
		let rows = query(
			r#"
			SELECT
				id
			FROM
				deployments
			ORDER BY
				id;
			"#,
		)
		.fetch_all(&self.state.database)
		.await?;

		Ok(rows
			.into_iter()
			.map(|row| row.get::<Uuid, _>("id"))
			.collect())
	}

	async fn get_deployment_info(
		&self,
		deployment_id: Uuid,
	) -> Result<GetDeploymentInfoResponse, ErrorType> {
		match &self.state.config.mode {
			RunnerMode::SelfHosted {
				password_pepper: _,
				jwt_secret: _,
			} => crate::routes::workspace::deployment::get_deployment_info(AppRequest {
				request: ProcessedApiRequest {
					path: GetDeploymentInfoPath {
						workspace_id: Uuid::nil(),
						deployment_id,
					},
					query: (),
					headers: GetDeploymentInfoRequestHeaders {
						authorization: BearerToken::from_str("").unwrap(),
						user_agent: UserAgent::from_str("").unwrap(),
					},
					body: GetDeploymentInfoRequestProcessed,
				},
				database: &mut self.state.database.begin().await?,
				config: self.state.config.clone().into_base(),
			})
			.await
			.map(|response| response.body),
			RunnerMode::Managed {
				workspace_id,
				runner_id: _,
				api_token: _,
			} => client::make_request(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id: *workspace_id,
						deployment_id,
					})
					.headers(GetDeploymentInfoRequestHeaders {
						authorization: self.authorization.clone(),
						user_agent: self.user_agent.clone(),
					})
					.query(())
					.body(GetDeploymentInfoRequest)
					.build(),
			)
			.await
			.map(|response| response.body)
			.map_err(|err| {
				debug!(
					"Failed to get deployment info for `{}`: {:?}",
					deployment_id, err
				);
				debug!("Retrying in 5 seconds");
				err.body.error
			}),
		}
	}
}
