use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

const IP_INFO_TOKEN: &str = "1a0c0cea381431";

/// Parsed response from ipinfo.io. We can't retrieve data from bogon or anycast
/// addresses.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum IpInfoParsed {
	Normal {
		loc: String,
	},

	/// Special case for IP addresses that don't provide the necessary
	/// metadata.
	///
	/// `bogon` should be true in this case.
	Bogon {},
}

#[operation(name = "ip-info")]
async fn handle(ctx: OperationContext<ip::info::Request>) -> GlobalResult<ip::info::Response> {
	let crdb = ctx.crdb().await?;

	// TODO: Handle situation where we can't find the location

	// Parse IP address
	let provider = unwrap!(ip::info::Provider::from_i32(ctx.provider));
	let ip = ctx.ip.parse::<std::net::IpAddr>()?;
	let ip_str = ip.to_string();
	tracing::info!(?ip, "looking up ip");

	// Fetch info
	let ip_info = match provider {
		ip::info::Provider::IpInfoIo => fetch_ip_info_io(&ctx, ctx.ts(), &ip_str).await?,
	};

	Ok(ip::info::Response { ip_info })
}

async fn fetch_ip_info_io(
	ctx: &OperationContext<ip::info::Request>,
	ts: i64,
	ip_str: &str,
) -> GlobalResult<Option<backend::net::IpInfo>> {
	// Read cached IP data if already exists
	let res = sql_fetch_optional!(
		[ctx, (Option<serde_json::Value>,)]
		"SELECT ip_info_io_data FROM db_ip_info.ips WHERE ip = $1",
		ip_str,
	)
	.await?;
	let ip_info_raw = if let Some(ip_info_raw) = res.and_then(|x| x.0) {
		tracing::info!("found cached ip info");
		ip_info_raw
	} else {
		// Fetch IP data from external service
		tracing::info!(?ip_str, "fetching fresh ip info");
		let ip_info_res = reqwest::get(format!(
			"https://ipinfo.io/{}?token={}",
			ip_str, IP_INFO_TOKEN
		))
		.await?;

		if !ip_info_res.status().is_success() {
			tracing::error!(status = ?ip_info_res.status(), "failed to fetch ip info, using fallback");

			bail!("ip info error")
		};

		let ip_info_raw = ip_info_res.json::<serde_json::Value>().await?;

		// Cache the IP info. This will be cached in Redis too, but this
		// prevents us from having to consume our ipinfo.io API quota once the
		// Redis cache expires.
		sql_execute!(
			[ctx]
			"UPSERT INTO db_ip_info.ips (ip, ip_info_io_data, ip_info_io_fetch_ts) VALUES ($1, $2, $3)",
			ip_str,
			&ip_info_raw,
			ts,
		)
		.await?;

		ip_info_raw
	};
	tracing::info!(?ip_info_raw, "acquired ip info");

	// Parse IP data
	let ip_info = serde_json::from_value::<IpInfoParsed>(ip_info_raw)?;
	let ip_info = match ip_info {
		IpInfoParsed::Normal { loc } => {
			// Parse latitude and longitude
			let loc_split = loc.split_once(',');
			let (latitude_raw, longitude_raw) = unwrap_ref!(loc_split, "failed to parse location");
			let latitude = latitude_raw.parse::<f64>()?;
			let longitude = longitude_raw.parse::<f64>()?;

			Some(backend::net::IpInfo {
				ip: ip_str.to_string(),
				latitude,
				longitude,
			})
		}
		IpInfoParsed::Bogon { .. } => {
			tracing::info!("bogon ip");
			None
		}
	};

	Ok(ip_info)
}
