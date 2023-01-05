use std::{fmt::Display, str::FromStr};

use api_models::{
	models::workspace::domain::DomainNameserverType,
	utils::{ResourceType, Uuid},
};
use chrono::{DateTime, Utc};
use eve_rs::AsError;

use crate::{error, query, query_as, utils::Error, Database};

pub struct Domain {
	pub id: Uuid,
	pub name: String,
	pub r#type: ResourceType,
}

pub struct PersonalDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceType,
}

pub struct WorkspaceDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceType,
	pub is_verified: bool,
	pub nameserver_type: DomainNameserverType,
	pub last_unverified: Option<DateTime<Utc>>,
}

impl WorkspaceDomain {
	pub fn is_ns_external(&self) -> bool {
		self.nameserver_type.is_external()
	}

	pub fn is_ns_internal(&self) -> bool {
		self.nameserver_type.is_internal()
	}
}

pub struct PatrControlledDomain {
	pub domain_id: Uuid,
	pub nameserver_type: DomainNameserverType,
	pub zone_identifier: String,
}

#[derive(sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "DNS_RECORD_TYPE", rename_all = "UPPERCASE")]
#[allow(clippy::upper_case_acronyms)]
pub enum DnsRecordType {
	A,
	AAAA,
	CNAME,
	MX,
	TXT,
}

impl Display for DnsRecordType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::A => write!(f, "A"),
			Self::AAAA => write!(f, "AAAA"),
			Self::CNAME => write!(f, "CNAME"),
			Self::MX => write!(f, "MX"),
			Self::TXT => write!(f, "TXT"),
		}
	}
}

impl FromStr for DnsRecordType {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_uppercase().as_str() {
			"A" => Ok(Self::A),
			"AAAA" => Ok(Self::AAAA),
			"CNAME" => Ok(Self::CNAME),
			"MX" => Ok(Self::MX),
			"TXT" => Ok(Self::TXT),
			_ => Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

pub struct DnsRecord {
	pub id: Uuid,
	pub record_identifier: String,
	pub domain_id: Uuid,
	pub name: String,
	pub r#type: DnsRecordType,
	pub value: String,
	pub priority: Option<i32>,
	pub ttl: i64,
	pub proxied: Option<bool>,
}

pub async fn initialize_domain_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing domain tables");

	query!(
		r#"
		CREATE TABLE domain_tld(
			tld TEXT
				CONSTRAINT domain_tld_pk PRIMARY KEY
				CONSTRAINT domain_tld_chk_is_length_valid CHECK(
					LENGTH(tld) >= 2 AND LENGTH(tld) <= 63
				)
				CONSTRAINT domain_tld_chk_is_tld_valid CHECK(
					tld ~ '^(([a-z0-9])|([a-z0-9][a-z0-9\-\.]*[a-z0-9]))$'
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE domain(
			id UUID CONSTRAINT domain_pk PRIMARY KEY,
			name TEXT NOT NULL
				CONSTRAINT domain_chk_name_is_valid CHECK(
					name ~ '^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$'
				),
			type RESOURCE_OWNER_TYPE NOT NULL,
			tld TEXT NOT NULL CONSTRAINT domain_fk_tld
					REFERENCES domain_tld(tld),
			deleted TIMESTAMPTZ,
			CONSTRAINT domain_chk_max_domain_name_length CHECK(
				(LENGTH(name) + LENGTH(tld)) < 255
			),
			CONSTRAINT domain_uq_name_type UNIQUE(id, type),
			CONSTRAINT domain_uq_id_type_deleted UNIQUE(id, type, deleted)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOMAIN_NAMESERVER_TYPE AS ENUM(
			'internal',
			'external'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE personal_domain(
			id UUID CONSTRAINT personal_domain_pk PRIMARY KEY,
			domain_type RESOURCE_OWNER_TYPE NOT NULL
				CONSTRAINT personal_domain_chk_domain_type CHECK(
					domain_type = 'personal'
				),
			deleted TIMESTAMPTZ
				CONSTRAINT personal_domain_chk_deletion CHECK(
					deleted IS NULL
				),
			CONSTRAINT personal_domain_fk_id_domain_type_deleted
				FOREIGN KEY(id, domain_type, deleted)
					REFERENCES domain(id, type, deleted)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace_domain(
			id UUID CONSTRAINT workspace_domain_pk PRIMARY KEY,
			domain_type RESOURCE_OWNER_TYPE NOT NULL
				CONSTRAINT workspace_domain_chk_domain_type CHECK(
					domain_type = 'business'
				),
			is_verified BOOLEAN NOT NULL,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL,
			last_unverified TIMESTAMPTZ,
			CONSTRAINT workspace_domain_uq_id_nameserver_type
				UNIQUE(id, nameserver_type),
			CONSTRAINT workspace_domain_fk_id_domain_type
				FOREIGN KEY(id, domain_type) REFERENCES domain(id, type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_domain_idx_is_verified
		ON
			workspace_domain
		(is_verified);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_controlled_domain(
			domain_id UUID NOT NULL
				CONSTRAINT patr_controlled_domain_pk PRIMARY KEY,
			zone_identifier TEXT NOT NULL,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
				CONSTRAINT patr_controlled_domain_chk_nameserver_type CHECK(
					nameserver_type = 'internal'
				),
			CONSTRAINT patr_controlled_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id, nameserver_type)	REFERENCES
					workspace_domain(id, nameserver_type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_controlled_domain(
			domain_id UUID NOT NULL
				CONSTRAINT user_controlled_domain_pk PRIMARY KEY,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
				CONSTRAINT user_controlled_domain_chk_nameserver_type CHECK(
					nameserver_type = 'external'
				),
			CONSTRAINT user_controlled_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id, nameserver_type)	REFERENCES
					workspace_domain(id, nameserver_type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DNS_RECORD_TYPE AS ENUM(
			'A',
			'MX',
			'TXT',
			'AAAA',
			'CNAME'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_domain_dns_record(
			id UUID CONSTRAINT patr_domain_dns_record_pk PRIMARY KEY,
			record_identifier TEXT NOT NULL,
			domain_id UUID NOT NULL,
			name TEXT NOT NULL
				CONSTRAINT patr_domain_dns_record_chk_name_is_valid CHECK(
					name ~ '^((\*)|((\*\.)?(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])))$' OR
					name = '@'
				),
			type DNS_RECORD_TYPE NOT NULL,
			value TEXT NOT NULL,
			priority INTEGER,
			ttl BIGINT NOT NULL,
			proxied BOOLEAN,
			CONSTRAINT patr_domain_dns_record_fk_domain_id
				FOREIGN KEY(domain_id)
					REFERENCES patr_controlled_domain(domain_id),
			CONSTRAINT patr_domain_dns_record_chk_values_valid CHECK(
				(
					type = 'MX' AND priority IS NOT NULL
				) OR (
					type != 'MX' AND priority IS NULL
				)
			),
			CONSTRAINT
				patr_domain_dns_record_uq_domain_id_name_type_value_priority
					UNIQUE(domain_id, name, type, value, priority),
			CONSTRAINT patr_domain_dns_record_chk_proxied_is_valid CHECK(
				(
					(type = 'A' OR type = 'AAAA' OR type = 'CNAME') AND
					proxied IS NOT NULL
				) OR
				(
					(type = 'MX' OR type = 'TXT') AND
					proxied IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_domain_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up domain tables initialization");
	query!(
		r#"
		ALTER TABLE workspace_domain
		ADD CONSTRAINT workspace_domain_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_domain_dns_record
		ADD CONSTRAINT patr_domain_dns_record_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			domain_uq_name_tld
		ON
			domain(name, tld)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn generate_new_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		// If it exists in the resource table, it can't be used
		// because workspace domains are a resource
		// If it exists in the domain table, it can't be used
		// since personal domains are a type of domains
		let exists = {
			query!(
				r#"
				SELECT
					*
				FROM
					resource
				WHERE
					id = $1;
				"#,
				uuid as _
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some()
		} || {
			query!(
				r#"
				SELECT
					id
				FROM
					domain
				WHERE
					id = $1;
				"#,
				uuid as _
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some()
		};

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn create_generic_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	domain_name: &str,
	tld: &str,
	domain_type: &ResourceType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			domain(
				id,
				name,
				type,
				tld
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		domain_id as _,
		domain_name,
		domain_type as _,
		tld,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_to_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	nameserver_type: &DomainNameserverType,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			workspace_domain(
				id,
				domain_type,
				is_verified,
				nameserver_type,
				last_unverified
			)
		VALUES
			($1, 'business', FALSE, $2, NULL);
		"#,
		domain_id as _,
		nameserver_type as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_to_personal_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_domain(
				id,
				domain_type
			)
		VALUES
			($1, 'personal');
		"#,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_patr_controlled_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	zone_identifier: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			patr_controlled_domain(
				domain_id,
				zone_identifier,
				nameserver_type
			)
		VALUES
			($1, $2, 'internal');
		"#,
		domain_id as _,
		zone_identifier,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_user_controlled_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_controlled_domain(
				domain_id,
				nameserver_type
			)
		VALUES
			($1, 'external');
		"#,
		domain_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_domains_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			CONCAT(domain.name, '.', domain.tld) as "name!",
			workspace_domain.id as "id: _",
			workspace_domain.domain_type as "domain_type: _",
			workspace_domain.is_verified,
			workspace_domain.nameserver_type as "nameserver_type: _",
			workspace_domain.last_unverified as "last_unverified"
		FROM
			domain
		INNER JOIN
			workspace_domain
		ON
			workspace_domain.id = domain.id
		INNER JOIN
			resource
		ON
			domain.id = resource.id
		WHERE
			resource.owner_id = $1 AND
			domain.deleted IS NULL;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_unverified_domains(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<(WorkspaceDomain, Option<String>)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			CONCAT(domain.name, '.', domain.tld) as "name!",
			workspace_domain.id as "id!: Uuid",
			workspace_domain.domain_type as "domain_type!: ResourceType",
			workspace_domain.is_verified as "is_verified!",
			workspace_domain.nameserver_type as "nameserver_type!: DomainNameserverType",
			workspace_domain.last_unverified as "last_unverified",
			patr_controlled_domain.zone_identifier as "zone_identifier?"
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		LEFT JOIN
			patr_controlled_domain
		ON
			patr_controlled_domain.domain_id = workspace_domain.id
		WHERE
			is_verified = FALSE AND
			domain.deleted IS NULL;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			WorkspaceDomain {
				id: row.id,
				name: row.name,
				domain_type: row.domain_type,
				is_verified: row.is_verified,
				nameserver_type: row.nameserver_type,
				last_unverified: row.last_unverified,
			},
			row.zone_identifier,
		)
	})
	.collect();

	Ok(rows)
}

pub async fn get_all_verified_domains(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<(WorkspaceDomain, Option<String>)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			CONCAT(domain.name, '.', domain.tld) as "name!",
			workspace_domain.id as "id!: Uuid",
			workspace_domain.domain_type as "domain_type!: ResourceType",
			workspace_domain.is_verified as "is_verified!",
			workspace_domain.nameserver_type as "nameserver_type!: DomainNameserverType",
			workspace_domain.last_unverified as "last_unverified",
			patr_controlled_domain.zone_identifier as "zone_identifier?"
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		LEFT JOIN
			patr_controlled_domain
		ON
			patr_controlled_domain.domain_id = workspace_domain.id
		WHERE
			is_verified = TRUE AND
			domain.deleted IS NULL;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			WorkspaceDomain {
				id: row.id,
				name: row.name,
				domain_type: row.domain_type,
				is_verified: row.is_verified,
				nameserver_type: row.nameserver_type,
				last_unverified: row.last_unverified,
			},
			row.zone_identifier,
		)
	})
	.collect();

	Ok(rows)
}

// TODO get the correct email based on permission
pub async fn get_notification_email_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
	let email = query!(
		r#"
		SELECT
			"user".*,
			CONCAT(domain.name, '.', domain.tld) as "name!"
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		INNER JOIN
			resource
		ON
			workspace_domain.id = resource.id
		INNER JOIN
			workspace
		ON
			resource.owner_id = workspace.id
		INNER JOIN
			"user"
		ON
			workspace.super_admin_id = "user".id
		WHERE
			workspace_domain.id = $1;
		"#,
		domain_id as _
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| {
		if let Some(email_local) = row.recovery_email_local {
			Ok(format!("{}@{}", email_local, row.name))
		} else {
			Err(sqlx::Error::RowNotFound)
		}
	})
	.transpose()?;

	Ok(email)
}

pub async fn delete_personal_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			personal_domain
		WHERE
			id = $1;
		"#,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn mark_domain_as_deleted(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			domain
		SET
			deleted = $2
		WHERE
			id = $1;
		"#,
		domain_id as _,
		deletion_time
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_workspace_domain_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Option<WorkspaceDomain>, sqlx::Error> {
	query_as!(
		WorkspaceDomain,
		r#"
		SELECT
			CONCAT(domain.name, '.', domain.tld) as "name!",
			workspace_domain.id as "id: _",
			workspace_domain.domain_type as "domain_type: _",
			workspace_domain.is_verified,
			workspace_domain.nameserver_type as "nameserver_type: _",
			workspace_domain.last_unverified as "last_unverified!: _"
		FROM
			workspace_domain
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		WHERE
			domain.id = $1 AND
			domain.deleted IS NULL;
		"#,
		domain_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_personal_domain_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Option<PersonalDomain>, sqlx::Error> {
	query_as!(
		PersonalDomain,
		r#"
		SELECT
			CONCAT(domain.name, '.', domain.tld) as "name!",
			personal_domain.id as "id: _",
			personal_domain.domain_type as "domain_type: _"
		FROM
			personal_domain
		INNER JOIN
			domain
		ON
			domain.id = personal_domain.id
		WHERE
			domain.id = $1;
		"#,
		domain_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_domain_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_name: &str,
) -> Result<Option<Domain>, sqlx::Error> {
	query_as!(
		Domain,
		r#"
		SELECT
			id as "id: _",
			CONCAT(domain.name, '.', domain.tld) as "name!",
			type as "type: _"
		FROM
			domain
		WHERE
			CONCAT(domain.name, '.', domain.tld) = $1 AND
			domain.deleted IS NULL;
		"#,
		domain_name
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_dns_records_by_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Vec<DnsRecord>, sqlx::Error> {
	query_as!(
		DnsRecord,
		r#"
		SELECT
			patr_domain_dns_record.id as "id: _",
			patr_domain_dns_record.record_identifier,
			patr_domain_dns_record.domain_id as "domain_id: _",
			patr_domain_dns_record.name,
			patr_domain_dns_record.type as "type: _",
			patr_domain_dns_record.value,
			patr_domain_dns_record.priority,
			patr_domain_dns_record.ttl,
			patr_domain_dns_record.proxied
		FROM
			patr_domain_dns_record
		INNER JOIN
			domain
		ON
			patr_domain_dns_record.domain_id = domain.id
		WHERE
			patr_domain_dns_record.domain_id = $1 AND
			domain.deleted IS NULL;
		"#,
		domain_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_patr_controlled_domain_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Option<PatrControlledDomain>, sqlx::Error> {
	query_as!(
		PatrControlledDomain,
		r#"
		SELECT
			domain_id as "domain_id: _",
			zone_identifier,
			nameserver_type as "nameserver_type: _"
		FROM
			patr_controlled_domain
		WHERE
			domain_id = $1;
		"#,
		domain_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_dns_record_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	record_id: &Uuid,
) -> Result<Option<DnsRecord>, sqlx::Error> {
	query_as!(
		DnsRecord,
		r#"
		SELECT
			patr_domain_dns_record.id as "id: _",
			patr_domain_dns_record.record_identifier,
			patr_domain_dns_record.domain_id as "domain_id: _",
			patr_domain_dns_record.name,
			patr_domain_dns_record.type as "type: _",
			patr_domain_dns_record.value,
			patr_domain_dns_record.priority,
			patr_domain_dns_record.ttl,
			patr_domain_dns_record.proxied
		FROM
			patr_domain_dns_record
		INNER JOIN
			domain
		ON
			patr_domain_dns_record.domain_id = domain.id
		WHERE
			patr_domain_dns_record.id = $1 AND
			domain.deleted IS NULL;
		"#,
		record_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

// ON CONFLICT reference: https://www.postgresqltutorial.com/postgresql-upsert/
pub async fn create_patr_domain_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	record_identifier: &str,
	domain_id: &Uuid,
	name: &str,
	r#type: &DnsRecordType,
	value: &str,
	priority: Option<i32>,
	ttl: i64,
	proxied: Option<bool>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			patr_domain_dns_record(
				id,
				record_identifier,
				domain_id,
				name,
				type,
				value,
				priority,
				ttl,
				proxied
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9);
		"#,
		id as _,
		record_identifier,
		domain_id as _,
		name,
		r#type as _,
		value,
		priority,
		ttl,
		proxied,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_patr_domain_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	value: Option<&str>,
	priority: Option<i32>,
	ttl: Option<i64>,
	proxied: Option<bool>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			patr_domain_dns_record
		SET
			value = $1,
			priority = $2,
			ttl = $3,
			proxied = $4
		WHERE
			id = $5;
		"#,
		value,
		priority,
		ttl,
		proxied,
		id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_patr_controlled_dns_record(
	connection: &mut <Database as sqlx::Database>::Connection,
	record_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			patr_domain_dns_record
		WHERE
			id = $1;
		"#,
		record_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_workspace_domain_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
	is_verified: bool,
	last_unverified: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace_domain
		SET
			is_verified = $2,
			last_unverified = $3
		WHERE
			id = $1;
		"#,
		domain_id as _,
		is_verified,
		last_unverified as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_top_level_domain_list(
	connection: &mut <Database as sqlx::Database>::Connection,
	tlds: &[String],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		WITH current_domains(tld) AS (
			SELECT
				UNNEST($1::TEXT[])
		),
		deprecated_domains(tld) AS (
			SELECT tld FROM domain_tld
			EXCEPT
			SELECT tld FROM current_domains
			EXCEPT
			SELECT tld FROM domain
			EXCEPT
			SELECT business_domain_tld FROM user_to_sign_up
		),
		inserted_domains(tld, action) AS (
			INSERT INTO domain_tld
				SELECT * FROM current_domains
			ON CONFLICT DO NOTHING
			RETURNING tld, 'inserted'
		),
		deleted_domains(tld, action) AS (
			DELETE FROM domain_tld
			WHERE EXISTS (
				SELECT
					tld
				FROM
					deprecated_domains
				WHERE
					deprecated_domains.tld = domain_tld.tld
			)
			RETURNING tld, 'deleted'
		)
		SELECT tld as "tld!", action as "action!" FROM inserted_domains
		UNION
		SELECT tld as "tld!", action as "action!" FROM deleted_domains;
		"#,
		&tlds[..],
	)
	.fetch_all(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_from_domain_tld_list(
	connection: &mut <Database as sqlx::Database>::Connection,
	tlds: &[String],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			domain_tld
		WHERE
			tld IN (
				SELECT UNNEST($1::TEXT[])
			);
		"#,
		&tlds[..],
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_dns_record_count_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<i64, sqlx::Error> {
	query!(
		r#"
		SELECT
			COUNT(*) as "count"
		FROM
			patr_domain_dns_record
		WHERE
			domain_id = $1;
		"#,
		domain_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.count.unwrap_or(0))
}

pub async fn update_dns_record_identifier(
	connection: &mut <Database as sqlx::Database>::Connection,
	record_id: &Uuid,
	record_identifier: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			patr_domain_dns_record
		SET
			record_identifier = $1
		WHERE
			id = $2;
		"#,
		record_identifier,
		record_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_users_with_domain_in_personal_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT DISTINCT
			user_id AS "user_id: Uuid"
		FROM
			personal_email
		WHERE
			domain_id = $1;
		"#,
		domain_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.user_id)
	.collect();

	Ok(rows)
}
