use jsonwebtoken::{errors::Error, DecodingKey, EncodingKey, TokenData, Validation};

#[derive(Serialize, Deserialize, Clone)]
pub struct AccessTokenData {
	pub iss: String,
	pub aud: String,
	pub iat: u64,
	pub typ: String,
	pub exp: u64,
	// TODO add more
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

	pub fn new(iat: u64, exp: u64) -> Self {
		AccessTokenData {
			iss: String::from("https://api.bytesonus.com"),
			aud: String::from("https://*.bytesonus.com"),
			iat,
			typ: String::from("accessToken"),
			exp,
		}
	}
}
