use std::collections::HashMap;

use jsonwebtoken::{
	errors::Error,
	DecodingKey,
	EncodingKey,
	TokenData,
	Validation,
};
use serde::{Deserialize, Serialize};

use crate::models::rbac::WorkspacePermissions;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenData {
	pub iss: String,
	pub aud: String,
	pub iat: u64,
	pub typ: String,
	pub exp: u64,
	pub workspaces: HashMap<String, WorkspacePermissions>,
	pub login_id: String,
	pub user: ExposedUserData,
	// Do we need to add more?
}

impl AccessTokenData {
	pub fn parse(token: String, key: &str) -> Result<AccessTokenData, Error> {
		let decode_key = DecodingKey::from_secret(key.as_ref());
		let TokenData { header: _, claims } = jsonwebtoken::decode(
			&token,
			&decode_key,
			&Validation {
				validate_exp: false,
				..Default::default()
			},
		)?;
		Ok(claims)
	}

	pub fn to_string(&self, key: &str) -> Result<String, Error> {
		jsonwebtoken::encode(
			&Default::default(),
			&self,
			&EncodingKey::from_secret(key.as_ref()),
		)
	}

	pub fn new(
		iat: u64,
		exp: u64,
		workspaces: HashMap<String, WorkspacePermissions>,
		login_id: String,
		user: ExposedUserData,
	) -> Self {
		AccessTokenData {
			iss: String::from("https://api.patr.cloud"),
			aud: String::from("https://*.patr.cloud"),
			iat,
			typ: String::from("accessToken"),
			exp,
			workspaces,
			login_id,
			user,
		}
	}
}

// Data about the user that can be exposed in the access token
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExposedUserData {
	pub id: Vec<u8>,
	pub username: String,
	pub first_name: String,
	pub last_name: String,
	pub created: u64,
}
