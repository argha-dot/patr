use api_macros::query;
use api_models::utils::{DateTime, Uuid};
use chrono::Utc;
use sqlx::query_as;

use super::ManagedDatabasePlan;
use crate::Database;

pub struct DeploymentPaymentHistory {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
	pub machine_type: Uuid,
	pub num_instance: i32,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct StaticSitesPaymentHistory {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub static_site_plan: Uuid,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct ManagedDatabasePaymentHistory {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub database_id: Uuid,
	pub db_plan: Uuid,
	pub start_time: DateTime<Utc>,
	pub deletion_time: Option<DateTime<Utc>>,
}

pub struct ManagedUrlPaymentHistory {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub url_count: i32,
	pub start_time: DateTime<Utc>,
	pub deletion_time: Option<DateTime<Utc>>,
}

pub struct SecretPaymentHistory {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub secret_count: i32,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct DockerRepoPaymentHistory {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub storage: i64,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct DomainPaymentHistory {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub domain_plan: Uuid,
	pub time: i64,
	pub start_time: DateTime<Utc>,
	pub stop_time: Option<DateTime<Utc>>,
}

pub struct Transaction {
	pub id: Uuid,
	pub month: i32,
	pub amount: i64,
	pub payment_intent_id: Option<String>,
	pub date: DateTime<Utc>,
	pub workspace_id: Uuid,
	pub transaction_type: TransactionType,
	pub payment_status: PaymentStatus,
}

pub struct PaymentMethod {
	pub id: String,
	pub workspace_id: Uuid,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "TRANSACTION_TYPE", rename_all = "lowercase")]
pub enum TransactionType {
	Bill,
	Credits,
	Payment,
}

#[derive(sqlx::Type, PartialEq)]
#[sqlx(type_name = "PAYMENT_STATUS", rename_all = "lowercase")]
pub enum PaymentStatus {
	Pending,
	Success,
	Failed,
}

pub async fn initialize_billing_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing billing tables");

	query!(
		r#"
		CREATE TYPE STATIC_SITE_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOMAIN_PLAN AS ENUM(
			'free',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_payment_history(
			id UUID CONSTRAINT deployment_payment_history_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			deployment_id UUID NOT NULL,
			machine_type UUID NOT NULL,
			num_instance INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS static_sites_payment_history(
			id UUID CONSTRAINT static_sites_payment_history_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			static_site_plan STATIC_SITE_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS managed_database_payment_history(
			id UUID CONSTRAINT managed_database_payment_history_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			database_id UUID NOT NULL,
			db_plan MANAGED_DATABASE_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			deletion_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS managed_url_payment_history(
			id UUID CONSTRAINT managed_url_payment_history_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			url_count INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			deletion_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS secrets_payment_history(
			id UUID CONSTRAINT secrets_payment_history_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			secret_count INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS docker_repo_payment_history(
			id UUID CONSTRAINT docker_repo_payment_history_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			storage BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS domain_payment_history(
			id UUID CONSTRAINT domain_payment_history_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			domain_plan DOMAIN_PLAN NOT NULL,
			time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS payment_method(
			payment_method_id TEXT CONSTRAINT payment_method_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE TRANSACTION_TYPE as ENUM(
			'bill',
			'credits',
			'payment'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PAYMENT_STATUS as ENUM(
			'success',
			'failed'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS transactions(
			id UUID CONSTRAINT transactions_pk PRIMARY KEY,
			month INTEGER NOT NULL,
			amount BIGINT NOT NULL,
			payment_intent_id TEXT,
			date TIMESTAMPTZ NOT NULL,
			workspace_id UUID NOT NULL,
			transaction_type TRANSACTION_TYPE NOT NULL,
			payment_status PAYMENT_STATUS NOT NULL
				CONSTRAINT transactions_payment_status_check CHECK (
					payment_status = 'success' AND
					TRANSACTION_TYPE = 'bill'
				)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_billing_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up billing tables initialization");

	query!(
		r#"
		ALTER TABLE deployment_payment_history 
		ADD CONSTRAINT deployment_payment_history_workspace_id_fk 
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_sites_payment_history
		ADD CONSTRAINT static_sites_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database_payment_history
		ADD CONSTRAINT managed_database_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url_payment_history
		ADD CONSTRAINT managed_url_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE secrets_payment_history
		ADD CONSTRAINT secrets_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_repo_payment_history
		ADD CONSTRAINT docker_repo_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain_payment_history
		ADD CONSTRAINT domain_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE payment_method
		ADD CONSTRAINT payment_method_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transaction
		ADD CONSTRAINT transaction_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_all_deployment_usage(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
) -> Result<Vec<DeploymentPaymentHistory>, sqlx::Error> {
	query_as!(
		DeploymentPaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			deployment_id as "deployment_id: _",
			machine_type as "machine_type: _",
			num_instance as "num_instance: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			deployment_payment_history
		WHERE
			workspace_id = $1 AND
			(
				(start_time > $2 AND stop_time IS NOT NULL) OR
				stop_time is NULL
			);
		"#,
		workspace_id as _,
		start_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_static_site_usages(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
) -> Result<Vec<StaticSitesPaymentHistory>, sqlx::Error> {
	query_as!(
		StaticSitesPaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			static_site_plan as "static_site_plan: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			static_sites_payment_history
		WHERE
			workspace_id = $1 AND
			(
				(start_time > $2 AND stop_time IS NOT NULL) OR
				stop_time is NULL
			);
		"#,
		workspace_id as _,
		start_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_database_usage(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
) -> Result<Vec<ManagedDatabasePaymentHistory>, sqlx::Error> {
	query_as!(
		ManagedDatabasePaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			database_id as "database_id: _",
			db_plan as "db_plan: _",
			start_time as "start_time: _",
			deletion_time as "deletion_time: _"
		FROM
			managed_database_payment_history
		WHERE
			workspace_id = $1 AND
			(
				(start_time > $2 AND deletion_time IS NOT NULL) OR
				deletion_time is NULL
			);
		"#,
		workspace_id as _,
		start_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_managed_url_usages(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
) -> Result<Vec<ManagedUrlPaymentHistory>, sqlx::Error> {
	query_as!(
		ManagedUrlPaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			url_count as "url_count: _",
			start_time as "start_time: _",
			deletion_time as "deletion_time: _"
		FROM
			managed_url_payment_history
		WHERE
			workspace_id = $1 AND
			(
				(start_time > $2 AND deletion_time IS NOT NULL) OR
				deletion_time is NULL
			);
		"#,
		workspace_id as _,
		start_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}
pub async fn get_all_docker_repository_usages(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
) -> Result<Vec<DockerRepoPaymentHistory>, sqlx::Error> {
	query_as!(
		DockerRepoPaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			storage as "storage: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			docker_repo_payment_history
		WHERE
			workspace_id = $1 AND
			(
				(start_time > $2 AND stop_time IS NOT NULL) OR
				stop_time is NULL
			);
		"#,
		workspace_id as _,
		start_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}
pub async fn get_all_domains_usages(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
) -> Result<Vec<DomainPaymentHistory>, sqlx::Error> {
	query_as!(
		DomainPaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			domain_plan as "domain_plan: _",
			time as "time: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			domain_payment_history
		WHERE
			workspace_id = $1 AND
			(
				(start_time > $2 AND stop_time IS NOT NULL) OR
				stop_time is NULL
			);
		"#,
		workspace_id as _,
		start_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_secrets_usages(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	start_date: &DateTime<Utc>,
) -> Result<Vec<SecretPaymentHistory>, sqlx::Error> {
	query_as!(
		SecretPaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			secret_count as "secret_count: _",
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			secrets_payment_history
		WHERE
			workspace_id = $1 AND
			(
				(start_time > $2 AND stop_time IS NOT NULL) OR
				stop_time is NULL
			);
		"#,
		workspace_id as _,
		start_date as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn create_transactions(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	id: &Uuid,
	month: i32,
	amount: i64,
	payment_intent_id: Option<&str>,
	date: &DateTime<Utc>,
	transaction_type: &TransactionType,
	payment_status: &PaymentStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			transactions(
				id,
				month,
				amount,
				payment_intent_id,
				date,
				workspace_id,
				transaction_type,
				payment_status
			)
			VALUES(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7,
				$8
			);
		"#,
		id as _,
		month as _,
		amount as _,
		payment_intent_id as _,
		date as _,
		workspace_id as _,
		transaction_type as _,
		payment_status as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn generate_new_transaction_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				id
			FROM
				transactions
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn get_last_bill_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_intent_id: String,
) -> Result<Option<Transaction>, sqlx::Error> {
	query_as!(
		Transaction,
		r#"
		SELECT
			id as "id: _",
			month,
			amount,
			payment_intent_id,
			date as "date: _",
			workspace_id as "workspace_id: _",
			transaction_type as "transaction_type: _",
			payment_status as "payment_status: _"
		FROM
			transactions
		WHERE
			workspace_id = $1 AND
			payment_intent_id = $2;
		"#,
		workspace_id as _,
		payment_intent_id,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_payment_methods_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<PaymentMethod>, sqlx::Error> {
	query_as!(
		PaymentMethod,
		r#"
		SELECT
			id,
			workspace_id as "workspace_id!: _"
		FROM
			payment_method
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_credits_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Transaction>, sqlx::Error> {
	query_as!(
		Transaction,
		r#"
		SELECT
			id as "id: _",
			month,
			amount,
			payment_intent_id,
			date as "date: _",
			workspace_id as "workspace_id: _",
			transaction_type as "transaction_type: _",
			payment_status as "payment_status: _"
		FROM
			transactions
		WHERE
			workspace_id = $1 AND
			transaction_type = 'credits';
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn generate_new_deployment_payment_history_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				id
			FROM
				deployment_payment_history
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn add_deployment_payment_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	machine_type: &Uuid,
	num_instance: i32,
	start_time: &DateTime<Utc>,
	stop_time: Option<&DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_payment_history
			(
				id,
				workspace_id,
				deployment_id,
				machine_type,
				num_instance,
				start_time,
				stop_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7
			);
		"#,
		id as _,
		workspace_id as _,
		deployment_id as _,
		machine_type as _,
		num_instance as _,
		start_time as _,
		stop_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_with_stop_deployment_payment_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_payment_history_id: &Uuid,
	machine_type: &Uuid,
	min_horizontal_scale: i32,
	stop_time: Option<&DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE deployment_payment_history
		SET
			machine_type = $2,
			num_instance = $3,
			stop_time = $4
		WHERE
			id = $1;
		"#,
		deployment_payment_history_id as _,
		machine_type as _,
		min_horizontal_scale as _,
		stop_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployment_payment_history_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
) -> Result<Option<DeploymentPaymentHistory>, sqlx::Error> {
	query_as!(
		DeploymentPaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			deployment_id as "deployment_id: _",
			machine_type as "machine_type: _",
			num_instance,
			start_time as "start_time: _",
			stop_time as "stop_time: _"
		FROM
			deployment_payment_history
		WHERE
			id = $1;
		"#,
		id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_managed_database_payment_history_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
) -> Result<Option<ManagedDatabasePaymentHistory>, sqlx::Error> {
	query_as!(
		ManagedDatabasePaymentHistory,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			database_id as "database_id: _",
			db_plan as "db_plan: _",
			start_time as "start_time: _",
			deletion_time as "deletion_time: _"
		FROM
			managed_database_payment_history
		WHERE
			id = $1;
		"#,
		id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_with_stop_database_payment_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	database_payment_history_id: &Uuid,
	stop_time: Option<&DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE deployment_payment_history
		SET
			stop_time = $2
		WHERE
			id = $1;
		"#,
		database_payment_history_id as _,
		stop_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn generate_new_database_payment_history_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				id
			FROM
				managed_database_payment_history
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn add_database_payment_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	id: &Uuid,
	workspace_id: &Uuid,
	database_id: &Uuid,
	db_plan: &ManagedDatabasePlan,
	start_time: &DateTime<Utc>,
	deletion_time: Option<&DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			managed_database_payment_history
			(
				id,
				workspace_id,
				database_id,
				db_plan,
				start_time,
				deletion_time
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6
			);
		"#,
		id as _,
		workspace_id as _,
		database_id as _,
		db_plan as _,
		start_time as _,
		deletion_time as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
