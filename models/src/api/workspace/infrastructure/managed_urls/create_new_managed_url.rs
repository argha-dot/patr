use crate::{
    prelude::*,
	utils::Uuid,
}; 
use super::ManagedUrlType;

macros::declare_api_endpoint!(
    /// Route to create a new managed URL
    CreateNewManagedUrl,
    POST "/workspace/:workspace_id/infrastructure/managed-url",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    request = {
        /// The sub domain of the URL
        pub sub_domain: String,
        /// The domain ID
        pub domain_id: Uuid,
        /// The path of the URL
        pub path: String,
        /// The URL type (Deployment, Static Site, Proxy or Redirect)
        pub url_type: ManagedUrlType,
    },
    response = {
        /// The new managed URL ID
        pub id: Uuid,
    }
);