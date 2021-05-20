use sqlx::Transaction;

use crate::{models::db_mapping::PortusTunnel, query, Database};

pub async fn initialize_portus_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing Portus tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS portus_tunnel(
			id BYTEA CONSTRAINT portus_tunnel_pk PRIMARY KEY,
			username VARCHAR(100) NOT NULL,
			ssh_port INTEGER NOT NULL
				CONSTRAINT portus_tunnel_chk_ssh_port_u16
					CHECK(ssh_port >= 0 AND ssh_port <= 65534),
			exposed_port INTEGER NOT NULL
				CONSTRAINT portus_tunnel_chk_exposed_port_u16
					CHECK(exposed_port >= 0 AND exposed_port <= 65534),
			name VARCHAR(50) NOT NULL,
			created BIGINT NOT NULL
				CONSTRAINT portus_tunnel_chk_created_unsigned
					CHECK(created >= 0)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}

pub async fn initialize_portus_post(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE portus_tunnel
		ADD CONSTRAINT portus_tunnel_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

// query to add user information with port and container details
pub async fn create_new_portus_tunnel(
	connection: &mut Transaction<'_, Database>,
	id: &[u8],
	username: &str,
	ssh_port: u16,
	exposed_port: u16,
	tunnel_name: &str,
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			portus_tunnel
		VALUES
			($1, $2, $3, $4, $5, $6);
		"#,
		id,
		username,
		ssh_port as i32,
		exposed_port as i32,
		tunnel_name,
		created as i64,
	)
	.execute(connection)
	.await?;

	Ok(())
}

// query to remove portus tunnel from database
pub async fn delete_portus_tunnel(
	connection: &mut Transaction<'_, Database>,
	tunnel_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			portus_tunnel
		WHERE
			id = $1;
		"#,
		tunnel_id
	)
	.execute(connection)
	.await?;
	Ok(())
}

/// function to check if container exists with the given tunnel name
pub async fn get_portus_tunnel_by_name(
	connection: &mut Transaction<'_, Database>,
	tunnel_name: &str,
) -> Result<Option<PortusTunnel>, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			portus_tunnel
		WHERE
			name = $1;
		"#,
		tunnel_name
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
		created: row.created as u64,
	});

	Ok(rows.next())
}

pub async fn get_portus_tunnel_by_tunnel_id(
	connection: &mut Transaction<'_, Database>,
	tunnel_id: &[u8],
) -> Result<Option<PortusTunnel>, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			portus_tunnel
		WHERE
			id = $1;
		"#,
		tunnel_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
		created: row.created as u64,
	});

	Ok(rows.next())
}

pub async fn is_portus_port_available(
	connection: &mut Transaction<'_, Database>,
	port: u16,
) -> Result<bool, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			portus_tunnel
		WHERE
			ssh_port = $1 OR
			exposed_port = $1;
		"#,
		port as i32
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
		created: row.created as u64,
	});

	Ok(rows.next().is_none())
}

pub async fn get_portus_tunnels_for_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
) -> Result<Vec<PortusTunnel>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT 
			portus_tunnel.*
		FROM 
			portus_tunnel
		INNER JOIN 
			resource 
		ON 
			resource.id = portus_tunnel.id
		WHERE
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| PortusTunnel {
		id: row.id,
		name: row.name,
		username: row.username,
		exposed_port: row.exposed_port as u16,
		ssh_port: row.ssh_port as u16,
		created: row.created as u64,
	})
	.collect();

	Ok(rows)
}
