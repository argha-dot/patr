use time::OffsetDateTime;

use super::DeploymentMetric;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get monitoring metrics like CPU, RAM and Disk usage
	/// for a deployment
	GetDeploymentMetric,
	GET "/workspace/:workspace_id/deployment/:deployment_id/metrics" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the metrics for
		pub deployment_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id,
			permission: Permission::Deployment(DeploymentPermission::View),
		}
	},
	query = {
		/// The time up until which the deployment metrics should be fetched
		pub end_time: Option<OffsetDateTime>,
		/// The limit of metrics to fetch. Defaults to 100
		pub limit: Option<u32>,
	},
	response = {
		/// The deployment metrics containing:
		/// pod_name - The name of the pod
		/// metrics - The metrics of the pod
		pub metrics: Vec<DeploymentMetric>
	}
);
