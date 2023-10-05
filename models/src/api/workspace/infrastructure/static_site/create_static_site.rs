use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
}; 
use super::StaticSiteDetails;

macros::declare_api_endpoint!(
    /// Definition of a route to create a new static site
    /// This route will allow users to upload a new index.html which would go live
    CreateStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site" {
        /// The workspace ID of the user
        pub workspace_id: Uuid
    },
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id,
		}
	},
    request = {
        /// The static site name
        pub name: String,
        /// Release message (eg: v1.0.0)
        pub message: String,
        /// The static site index.html file
        pub file: Option<String>,
        /// Static site details which included metrics, etc
        pub static_site_details: StaticSiteDetails,
    },
    response = {
        /// The new static site ID
        pub id: Uuid
    }
);