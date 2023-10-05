use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
}; 
use super::ManagedUrlType;

macros::declare_api_endpoint!(
    /// Route to update a managed URL configurations
    UpdateManagedUrl,
    POST "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id" {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The managed URL to be deleted
        pub managed_url_id: Uuid,
    },
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
            extract_resource_id: |req| req.path.managed_url_id
        }
	},
    request = {
        /// The new path of the updated URL
        pub path: String,
        /// The new type of the updated URL which can be
        /// Deployment, Static Site, Proxy or Redirect
        pub url_type: ManagedUrlType,
    },
);