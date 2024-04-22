use std::collections::BTreeMap;

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to create a new role
	UpdateRole,
	PATCH "/workspace/:workspace_id/rbac/role/:role_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the role to update
		pub role_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// The updated name of the role
		#[preprocess(none)]
		pub name: Option<String>,
		/// The updated description of the role
		#[preprocess(none)]
		pub description: Option<String>,
		/// The updated list of permission this role has
		#[preprocess(none)]
		pub resource_permissions: Option<BTreeMap<Uuid, Vec<Uuid>>>,
		/// The updated list of permissions this role has on what resource types
		#[preprocess(none)]
		pub resource_type_permissions: Option<BTreeMap<Uuid, Vec<Uuid>>>
	}
);
