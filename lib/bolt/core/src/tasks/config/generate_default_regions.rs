use anyhow::*;
use serde::Deserialize;
use tokio::fs;
use toml_edit::value;
use uuid::Uuid;

use crate::context;

struct UniversalRegion {
	provider: String,
	provider_region: String,
	debug: String,
}

/// Fetches all regions from the supported cloud providers and adds missing
/// regions to the `lib/bolt/config/default_regions.toml` file.
pub async fn generate_default_regions() -> Result<()> {
	let term = rivet_term::terminal();

	let project_root = context::ProjectContextData::seek_project_root().await;
	let default_regions_path = project_root.join("lib/bolt/config/default_regions.toml");

	let toml = fs::read_to_string(&default_regions_path).await?;
	let mut doc = toml.parse::<toml_edit::Document>()?;

	// Insert missing regions
	let universal_regions = fetch_linode_universal_regions().await?;
	'outer: for region in universal_regions {
		// Check if the region already exists
		for (_, v) in doc.as_table() {
			if v["provider"].as_str() == Some(region.provider.as_str())
				&& v["provider_region"].as_str() == Some(region.provider_region.as_str())
			{
				continue 'outer;
			}
		}

		// Prompt for new region
		let name_id = rivet_term::prompt::PromptBuilder::default()
			.message(format!("{}:{}", region.provider, region.provider_region))
			.docs(region.debug)
			.build()?
			.string(&term)
			.await?;

		let netnum = find_max_netnum(&doc) + 1;

		let mut table = toml_edit::table();
		table["id"] = value(Uuid::new_v4().to_string());
		table["provider"] = value(&region.provider);
		table["provider_region"] = value(&region.provider_region);
		table["netnum"] = value(netnum);
		doc.as_table_mut()[&name_id] = table;

		rivet_term::status::info("Added region", "");

		// Save to file after each step
		fs::write(&default_regions_path, doc.to_string()).await?;
	}

	Ok(())
}

async fn fetch_linode_universal_regions() -> Result<Vec<UniversalRegion>> {
	#[derive(Deserialize, Debug)]
	#[allow(unused)]
	struct Region {
		id: String,
		label: String,
	}

	#[derive(Deserialize, Debug)]
	struct LinodeRegionsResponse {
		data: Vec<Region>,
	}

	// Build client
	let linode_token = std::env::var("RIVET_LINODE_TOKEN").context("missing RIVET_LINODE_TOKEN")?;
	let mut headers = reqwest::header::HeaderMap::new();
	headers.insert(
		reqwest::header::AUTHORIZATION,
		reqwest::header::HeaderValue::from_str(&format!("Bearer {linode_token}"))?,
	);
	let client = reqwest::Client::builder()
		.default_headers(headers)
		.build()?;

	// Fetch regions
	let resp = client
		.get("https://api.linode.com/v4/regions")
		.send()
		.await?;
	let regions: LinodeRegionsResponse = resp.json().await?;

	// Convert to universal regions
	let universal_regions = regions
		.data
		.into_iter()
		.map(|x| UniversalRegion {
			debug: format!("{x:?}"),
			provider: "linode".into(),
			provider_region: x.id,
		})
		.collect::<Vec<_>>();

	Ok(universal_regions)
}

/// Finds the highest netnum in the regions configs.
fn find_max_netnum(doc: &toml_edit::Document) -> i64 {
	doc.as_table()
		.iter()
		.map(|(_, v)| v["netnum"].as_integer().unwrap_or(0))
		.max()
		.unwrap_or(0)
}
