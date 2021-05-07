pub struct Organisation {
	pub id: Vec<u8>,
	pub name: String,
	pub super_admin_id: Vec<u8>,
	pub active: bool,
	pub created: u64,
}

// Uncomment this when generic_domain realted query is in use
// pub struct GenericDomain {
// 	pub id: Vec<u8>,
// 	pub name: String,
// 	pub domain_type: String,
// }

pub struct OrganisationDomain {
	pub id: Vec<u8>,
	pub name: String,
	pub is_verified: bool,
}

pub struct PersonalDomain {
	pub id: Vec<u8>,
	pub name: String,
}

pub struct Application {
	pub id: Vec<u8>,
	pub name: String,
}

// struct to store information regarding the version for an application.
pub struct ApplicationVersion {
	pub application_id: Vec<u8>,
	pub version: String,
}
