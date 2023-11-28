use crate::{prelude::*, utils::BearerToken};
use time::OffsetDateTime;
use super::Region;

macros::declare_api_endpoint!(
	/// Route to get region information
	GetRegionInfo,
	GET "/workspace/:workspace_id/region/:region_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The region ID 
		pub region_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.region_id
		}
	},
	response = {
		/// The region information containing:
		///     name - The name of the region
		///     cloud_provider - The cloud provider the region is of
		///     status - The status of the region
		///     r#type - The region type
		pub region: WithId<Region>,
		/// The logs
		pub message_log: Option<String>,
		/// The time when the region was disconnected
		pub disconnected_at: Option<OffsetDateTime>
	}
);
