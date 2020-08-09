
#[allow(dead_code)]
pub mod rbac;
#[allow(dead_code)]
pub mod errors;
pub mod user;
pub mod access_token_data;

/*
New:

Users have roles
Users belong to an organisation through a role
Roles have permissions on a resource
Resources belong to a user, or an organisation


When validating a request:
- Check how the user has access to the resouce.
- If the user has access to the resource directly,
	- Check if their personal roles grant the required permissions
- If the user has access to the resource through an organisation,
	- Check if their organisation roles grant the required permissions


Each resource must be "owned" by someone or the other.
There can't be a resouce that doesn't have an owner.
2 ways to do this:
- The "owner" role cannot be removed from a resource. 
  Can be transferred, maybe, but no.
	Pros:
	- Fits in well with rbac middlewares. Can be done in the same SQL query
	Cons:
	- In case, by mistake, the role is removed, we now have a dangling resource
- The database schema for a resource has a "owner" field
  that points to either a user or an org
	Pros:
	- Dangling resources can't exist. They NEED to be owned by someone as per the db schema
	Cons:
	- Need to do a more complex query to check if owner field as a valid role


-------















Resources <- require -> (one or many) Permissions.
Roles <- are collections of -> (one or many) Permissions.
Users <- can have -> (one or many) Roles.

The tables for such a model would be:
permission
role
user
role_permission
user_role

Permission model:

Users:
- UserID
- Roles[]

Organizations:
- Users[]

Roles:
- RoleID
- Permissions[]

Permission:
- PermissionID
- PermissionType

Resources:
- PermissionsRequired[]

*/