use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Server {
	server_id: Uuid,
	public_ip: String,
}

impl From<Server> for cluster::server_resolve_for_ip::response::Server {
	fn from(value: Server) -> Self {
		cluster::server_resolve_for_ip::response::Server {
			server_id: Some(value.server_id.into()),
			public_ip: value.public_ip,
		}
	}
}

#[operation(name = "cluster-server-resolve-for-ip")]
pub async fn handle(
	ctx: OperationContext<cluster::server_resolve_for_ip::Request>,
) -> GlobalResult<cluster::server_resolve_for_ip::Response> {
	let servers = sql_fetch_all!(
		[ctx, Server]
		"
		SELECT
			server_id, public_ip
		FROM db_cluster.servers
		WHERE public_ip = ANY($1)
		",
		&ctx.ips
	)
	.await?;

	Ok(cluster::server_resolve_for_ip::Response {
		servers: servers.into_iter().map(Into::into).collect::<Vec<_>>(),
	})
}
