use crate::prelude::*;
use std::{
	fmt::Display, 
	net::{Ipv4Addr, Ipv6Addr},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod add_dns_record;
mod add_domain_to_workspace;
mod delete_dns_record;
mod delete_domain_in_workspace;
mod get_domain_dns_record;
mod get_domain_info_in_workspace;
mod get_domains_for_workspace;
mod is_domain_personal;
mod update_domain_dns_record;
mod verify_domain_in_workspace;

pub use self::{
	add_dns_record::*,
	add_domain_to_workspace::*,
	delete_dns_record::*,
	delete_domain_in_workspace::*,
	get_domain_dns_record::*,
	get_domain_info_in_workspace::*,
	get_domains_for_workspace::*,
	is_domain_personal::*,
	update_domain_dns_record::*,
	verify_domain_in_workspace::*,
};

/// The domain metadeta information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
	/// The name of the domain 
	pub name: String,
	/// Last verified time of the domain
	pub last_unverified: Option<OffsetDateTime>,
}

/// The domain information in a workspace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDomain {
	/// The domain metadata
	#[serde(flatten)]
	pub domain: Domain,
	/// Whether or not the domain is verified
	pub is_verified: bool,
	/// The domain nameserver type
	pub nameserver_type: DomainNameserverType,
}

impl WorkspaceDomain {
	/// To check if the nameserver is internal
	pub fn is_ns_internal(&self) -> bool {
		self.nameserver_type.is_internal()
	}

	/// To check if the nameserver is external
	pub fn is_ns_external(&self) -> bool {
		self.nameserver_type.is_external()
	}
}

/// Patr domain information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatrControlledDomain {
	/// The domain ID
	pub domain_id: Uuid,
	/// The domain nameserver type 
	pub nameserver_type: DomainNameserverType,
	/// The domain zone identifier
	pub zone_identifier: String,
}

/// The DNS record type of a domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
#[serde(tag = "type")]
pub enum DnsRecordValue {
	/// A
	A {
		/// The target address
		target: Ipv4Addr,
		/// Whether proxied or not
		proxied: bool 
	},
	/// MX
	MX {
		/// The priorty
		priority: u16, 
		/// The target address
		target: String 
	},
	/// TXT
	TXT {
		/// The target address
		target: String 
	},
	/// AAAA
	AAAA {
		/// The target address
		target: Ipv6Addr,
		/// Whether proxied or not
		proxied: bool 
	},
	/// CNAME
	CNAME {
		/// The target address
		target: String,
		/// Whether proxied or not
		proxied: bool 
	},
}

impl DnsRecordValue {
	/// To check if the record is of type A
	pub fn is_a_record(&self) -> bool {
		matches!(self, DnsRecordValue::A { .. })
	}

	/// To check if the record is of type AAAA
	pub fn is_aaaa_record(&self) -> bool {
		matches!(self, DnsRecordValue::AAAA { .. })
	}

	/// To check if the record is of type CNAME
	pub fn is_cname_record(&self) -> bool {
		matches!(self, DnsRecordValue::CNAME { .. })
	}

	/// To check if the record is of type MX
	pub fn is_mx_record(&self) -> bool {
		matches!(self, DnsRecordValue::MX { .. })
	}

	/// To check if the record is of type TXT
	pub fn is_txt_record(&self) -> bool {
		matches!(self, DnsRecordValue::TXT { .. })
	}

	/// To return as of type some
	pub fn as_a_record(&self) -> Option<(&Ipv4Addr, bool)> {
		match self {
			DnsRecordValue::A { target, proxied } => Some((target, *proxied)),
			_ => None,
		}
	}

	/// To return as of type some
	pub fn as_aaaa_record(&self) -> Option<(&Ipv6Addr, bool)> {
		match self {
			DnsRecordValue::AAAA { target, proxied } => {
				Some((target, *proxied))
			}
			_ => None,
		}
	}

	/// To return as of type some
	pub fn as_cname_record(&self) -> Option<(&str, bool)> {
		match self {
			DnsRecordValue::CNAME { target, proxied } => {
				Some((target, *proxied))
			}
			_ => None,
		}
	}

	/// To return as of type some
	pub fn as_mx_record(&self) -> Option<(u16, &str)> {
		match self {
			DnsRecordValue::MX { priority, target } => {
				Some((*priority, target))
			}
			_ => None,
		}
	}

	/// To return as of type some
	pub fn as_txt_record(&self) -> Option<&str> {
		match self {
			DnsRecordValue::TXT { target } => Some(target),
			_ => None,
		}
	}
}

impl Display for DnsRecordValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::A { .. } => write!(f, "A"),
			Self::AAAA { .. } => write!(f, "AAAA"),
			Self::CNAME { .. } => write!(f, "CNAME"),
			Self::MX { .. } => write!(f, "MX"),
			Self::TXT { .. } => write!(f, "TXT"),
		}
	}
}
