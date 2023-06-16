#![allow(unused)]

use anyhow::*;
use serde_json::json;

use crate::{context::ProjectContext, dep::terraform, utils::command_helper::CommandHelper};

// Export the state from the previous repo version with:
//
// for x in tls s3 infra grafana; do (cd tf/$x/ && terraform state pull) > /tmp/migrate-staging/$x.tfstate; done

// Remove unused Vault resources with:
//
// terraform state list | grep vault | xargs -L 1 -d'\n' terraform state rm
//
// This needs to be done in:
// - infra/tf/tls
// - infra/tf/nomad

pub async fn run(ctx: &ProjectContext) -> Result<()> {
	crate::tasks::gen::generate_project(&ctx).await;
	migrate_nebula(ctx).await?;
	migrate_master_cluster(ctx).await?;
	// migrate_pools(ctx).await?;
	// migrate_dns(ctx).await?;
	// migrate_cloudflare_workers(ctx).await?;
	// migrate_cloudflare_tunnels(ctx).await?;
	migrate_s3(ctx).await?;
	Ok(())
}

async fn migrate_nebula(ctx: &ProjectContext) -> Result<()> {
	let src_infra = read_src_state(ctx, "infra").await?;

	let mut dst = pull_dst_state(ctx, "nebula").await?;

	copy_resource(
		ctx,
		&src_infra,
		(None, "managed", "nebula_ca", "main"),
		&mut dst,
		(None, "managed", "nebula_ca", "main"),
	)
	.await?;

	push_dst_state(ctx, "nebula", dst).await?;

	Ok(())
}

async fn migrate_master_cluster(ctx: &ProjectContext) -> Result<()> {
	let src_infra = read_src_state(ctx, "infra").await?;

	let mut dst = pull_dst_state(ctx, "master_cluster").await?;

	// MARK: Nebula lighthouse server
	{
		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.nebula_lighthouse"),
				"managed",
				"random_string",
				"nebula_lighthouse_root_pass",
			),
			&mut dst,
			(
				Some("module.nebula_lighthouse_server"),
				"managed",
				"random_string",
				"server_root_pass",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.nebula_lighthouse"),
				"managed",
				"linode_sshkey",
				"nebula_lighthouse",
			),
			&mut dst,
			(
				Some("module.nebula_lighthouse_server"),
				"managed",
				"linode_sshkey",
				"server",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.nebula_lighthouse"),
				"managed",
				"linode_instance",
				"nebula_lighthouse",
			),
			&mut dst,
			(
				Some("module.nebula_lighthouse_server"),
				"managed",
				"linode_instance",
				"server",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.nebula_lighthouse"),
				"managed",
				"linode_instance_disk",
				"nebula_lighthouse_boot",
			),
			&mut dst,
			(
				Some("module.nebula_lighthouse_server"),
				"managed",
				"linode_instance_disk",
				"server_boot",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.nebula_lighthouse"),
				"managed",
				"linode_instance_disk",
				"nebula_lighthouse_swap",
			),
			&mut dst,
			(
				Some("module.nebula_lighthouse_server"),
				"managed",
				"linode_instance_disk",
				"server_swap",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.nebula_lighthouse"),
				"managed",
				"linode_instance_config",
				"nebula_lighthouse_boot_config",
			),
			&mut dst,
			(
				Some("module.nebula_lighthouse_server"),
				"managed",
				"linode_instance_config",
				"server_boot_config",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.nebula_lighthouse"),
				"managed",
				"linode_firewall",
				"nebula_lighthouse",
			),
			&mut dst,
			(
				Some("module.nebula_lighthouse_server"),
				"managed",
				"linode_firewall",
				"server",
			),
		)
		.await?;
	}

	// MARK: Salt master
	{
		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.salt_master"),
				"managed",
				"random_string",
				"salt_master_root_pass",
			),
			&mut dst,
			(
				Some("module.salt_master_server"),
				"managed",
				"random_string",
				"server_root_pass",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.salt_master"),
				"managed",
				"linode_sshkey",
				"salt_master",
			),
			&mut dst,
			(
				Some("module.salt_master_server"),
				"managed",
				"linode_sshkey",
				"server",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.salt_master"),
				"managed",
				"linode_instance",
				"salt_master",
			),
			&mut dst,
			(
				Some("module.salt_master_server"),
				"managed",
				"linode_instance",
				"server",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.salt_master"),
				"managed",
				"linode_instance_disk",
				"salt_master_boot",
			),
			&mut dst,
			(
				Some("module.salt_master_server"),
				"managed",
				"linode_instance_disk",
				"server_boot",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.salt_master"),
				"managed",
				"linode_instance_disk",
				"salt_master_swap",
			),
			&mut dst,
			(
				Some("module.salt_master_server"),
				"managed",
				"linode_instance_disk",
				"server_swap",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.salt_master"),
				"managed",
				"linode_instance_config",
				"salt_master_boot_config",
			),
			&mut dst,
			(
				Some("module.salt_master_server"),
				"managed",
				"linode_instance_config",
				"server_boot_config",
			),
		)
		.await?;

		copy_resource(
			ctx,
			&src_infra,
			(
				Some("module.salt_master"),
				"managed",
				"linode_firewall",
				"salt_master",
			),
			&mut dst,
			(
				Some("module.salt_master_server"),
				"managed",
				"linode_firewall",
				"server",
			),
		)
		.await?;
	}

	push_dst_state(ctx, "master_cluster", dst).await?;

	Ok(())
}

async fn migrate_pools(ctx: &ProjectContext) -> Result<()> {
	let src_infra = read_src_state(ctx, "infra").await?;

	let mut dst = pull_dst_state(ctx, "pools").await?;

	// We'll create a separate set of pools and create those then remove the old ones
	// copy_resource_with_index_to_module_with_index(
	// 	ctx,
	// 	&src_infra,
	// 	(None, "managed", "random_string", "server_root_pass"),
	// 	&mut dst,
	// 	(
	// 		"module.servers",
	// 		"managed",
	// 		"random_string",
	// 		"server_root_pass",
	// 	),
	// )
	// .await?;

	// copy_resource_with_index_to_module_with_index(
	// 	ctx,
	// 	&src_infra,
	// 	(None, "managed", "linode_instance", "server"),
	// 	&mut dst,
	// 	("module.servers", "managed", "linode_instance", "server"),
	// )
	// .await?;

	// copy_resource_with_index_to_module_with_index(
	// 	ctx,
	// 	&src_infra,
	// 	(None, "managed", "linode_instance_disk", "server_boot"),
	// 	&mut dst,
	// 	(
	// 		"module.servers",
	// 		"managed",
	// 		"linode_instance_disk",
	// 		"server_boot",
	// 	),
	// )
	// .await?;

	// copy_resource_with_index_to_module_with_index(
	// 	ctx,
	// 	&src_infra,
	// 	(None, "managed", "linode_instance_disk", "server_swap"),
	// 	&mut dst,
	// 	(
	// 		"module.servers",
	// 		"managed",
	// 		"linode_instance_disk",
	// 		"server_swap",
	// 	),
	// )
	// .await?;

	// copy_resource_with_index_to_module_with_index(
	// 	ctx,
	// 	&src_infra,
	// 	(
	// 		None,
	// 		"managed",
	// 		"linode_instance_config",
	// 		"server_boot_config",
	// 	),
	// 	&mut dst,
	// 	(
	// 		"module.servers",
	// 		"managed",
	// 		"linode_instance_config",
	// 		"server_boot_config",
	// 	),
	// )
	// .await?;

	// TODO: Volumes
	// TODO: Firewalls
	// TODO: SSH keys

	push_dst_state(ctx, "pools", dst).await?;

	Ok(())
}

async fn migrate_dns(ctx: &ProjectContext) -> Result<()> {
	let src_infra = read_src_state(ctx, "infra").await?;

	let mut dst = pull_dst_state(ctx, "dns").await?;

	copy_resource(
		ctx,
		&src_infra,
		(None, "managed", "cloudflare_certificate_pack", "rivet_gg"),
		&mut dst,
		(None, "managed", "cloudflare_certificate_pack", "rivet_gg"),
	)
	.await?;

	copy_resource(
		ctx,
		&src_infra,
		(None, "managed", "cloudflare_certificate_pack", "rivet_game"),
		&mut dst,
		(None, "managed", "cloudflare_certificate_pack", "rivet_game"),
	)
	.await?;

	copy_resource(
		ctx,
		&src_infra,
		(None, "managed", "cloudflare_record", "rivet_gg"),
		&mut dst,
		(None, "managed", "cloudflare_record", "rivet_gg"),
	)
	.await?;

	push_dst_state(ctx, "dns", dst).await?;

	Ok(())
}

async fn migrate_cloudflare_workers(ctx: &ProjectContext) -> Result<()> {
	let src_infra = read_src_state(ctx, "infra").await?;

	let mut dst = pull_dst_state(ctx, "cloudflare_workers").await?;

	copy_resource(
		ctx,
		&src_infra,
		(None, "managed", "cloudflare_worker_script", "request_meta"),
		&mut dst,
		(None, "managed", "cloudflare_worker_script", "request_meta"),
	)
	.await?;

	copy_resource(
		ctx,
		&src_infra,
		(None, "managed", "cloudflare_worker_route", "my_route"),
		&mut dst,
		(
			None,
			"managed",
			"cloudflare_worker_route",
			"request_meta_route",
		),
	)
	.await?;

	push_dst_state(ctx, "cloudflare_workers", dst).await?;

	Ok(())
}

async fn migrate_cloudflare_tunnels(ctx: &ProjectContext) -> Result<()> {
	let src_infra = read_src_state(ctx, "infra").await?;

	let mut dst = pull_dst_state(ctx, "cloudflare_tunnels").await?;

	copy_resource_module_prefix(
		ctx,
		&src_infra,
		(
			"module.cloudflare_tunnels",
			"managed",
			"random_id",
			"tunnel_secret",
		),
		&mut dst,
		("managed", "random_id", "tunnel_secret"),
	)
	.await?;

	copy_resource_module_prefix(
		ctx,
		&src_infra,
		(
			"module.cloudflare_tunnels",
			"managed",
			"cloudflare_argo_tunnel",
			"tunnel",
		),
		&mut dst,
		("managed", "cloudflare_argo_tunnel", "tunnel"),
	)
	.await?;

	copy_resource_module_prefix(
		ctx,
		&src_infra,
		(
			"module.cloudflare_tunnels",
			"managed",
			"cloudflare_record",
			"tunnel",
		),
		&mut dst,
		("managed", "cloudflare_record", "tunnel"),
	)
	.await?;

	copy_resource_module_prefix(
		ctx,
		&src_infra,
		(
			"module.cloudflare_tunnels",
			"managed",
			"cloudflare_access_application",
			"tunnel",
		),
		&mut dst,
		("managed", "cloudflare_access_application", "tunnel"),
	)
	.await?;

	copy_resource_module_prefix(
		ctx,
		&src_infra,
		(
			"module.cloudflare_tunnels",
			"managed",
			"cloudflare_access_policy",
			"allow",
		),
		&mut dst,
		("managed", "cloudflare_access_policy", "allow"),
	)
	.await?;

	copy_resource_module_prefix(
		ctx,
		&src_infra,
		(
			"module.cloudflare_tunnels",
			"managed",
			"cloudflare_access_policy",
			"service_auth",
		),
		&mut dst,
		("managed", "cloudflare_access_policy", "service_auth"),
	)
	.await?;

	push_dst_state(ctx, "cloudflare_tunnels", dst).await?;

	Ok(())
}

async fn migrate_s3(ctx: &ProjectContext) -> Result<()> {
	let src_infra = read_src_state(ctx, "s3").await?;

	let mut dst = pull_dst_state(ctx, "s3_backblaze").await?;

	copy_resource(
		ctx,
		&src_infra,
		(None, "managed", "b2_bucket", "bucket"),
		&mut dst,
		(None, "managed", "b2_bucket", "bucket"),
	)
	.await?;

	push_dst_state(ctx, "s3_backblaze", dst).await?;

	Ok(())
}

async fn read_src_state(ctx: &ProjectContext, plan_id: &str) -> Result<serde_json::Value> {
	let src_state_str =
		tokio::fs::read_to_string(&format!("/tmp/migrate-{}/{plan_id}.tfstate", ctx.ns_id()))
			.await?;
	let src_state = serde_json::from_str::<serde_json::Value>(&src_state_str)?;
	Ok(src_state)
}

async fn pull_dst_state(ctx: &ProjectContext, plan_id: &str) -> Result<serde_json::Value> {
	// TODO: Cache this
	// Pull current state
	let mut pull_cmd = terraform::cli::build_command(ctx, plan_id).await;
	pull_cmd.arg("state").arg("pull");

	// Read state
	let dst_state_json = pull_cmd.exec_string().await?;
	let dst_state = serde_json::from_str::<serde_json::Value>(&dst_state_json)?;

	Ok(dst_state)
}

async fn push_dst_state(
	ctx: &ProjectContext,
	plan_id: &str,
	mut state: serde_json::Value,
) -> Result<()> {
	// Add one to serial
	state["serial"] = json!(state["serial"].as_u64().unwrap() + 1);

	println!("Writing {plan_id}:\n{state:#?}");

	let state_buf = serde_json::to_vec(&state)?;

	// Push the new state
	let mut pull_cmd = terraform::cli::build_command(ctx, plan_id).await;
	pull_cmd.arg("state").arg("push").arg("-");
	pull_cmd.exec_stdin(state_buf).await?;

	Ok(())
}

async fn copy_resource(
	ctx: &ProjectContext,
	src_state: &serde_json::Value,
	(src_module, src_mode, src_ty, src_name): (Option<&str>, &str, &str, &str),
	dst_state: &mut serde_json::Value,
	(dst_module, dst_mode, dst_ty, dst_name): (Option<&str>, &str, &str, &str),
) -> Result<()> {
	// Find resource
	let mut src_resource = src_state["resources"]
		.as_array()
		.unwrap()
		.iter()
		.find(|r| {
			let r = r.as_object().unwrap();
			let r_module = r.get("module").and_then(|m| m.as_str());
			let r_mode = r.get("mode").unwrap().as_str().unwrap();
			let r_ty = r.get("type").unwrap().as_str().unwrap();
			let r_name = r.get("name").unwrap().as_str().unwrap();
			r_module == src_module && r_mode == src_mode && r_ty == src_ty && r_name == src_name
		})
		.context(format!(
			"resource not found: {src_module:?}.{src_mode}.{src_ty}.{src_name}"
		))?
		.clone();
	println!("Found resource: {src_module:?}.{src_mode}.{src_ty}.{src_name}");

	// Remove all dependencies since they may contain invalid references. This
	// will be re-populated when applied.
	for instance in src_resource
		.get_mut("instances")
		.unwrap()
		.as_array_mut()
		.unwrap()
	{
		instance["dependencies"] = json!([]);
	}

	// Check if already has resource
	let existing_dst_resource = dst_state
		.get_mut("resources")
		.unwrap()
		.as_array_mut()
		.unwrap()
		.iter_mut()
		.find(|r| {
			let r = r.as_object().unwrap();
			let r_module = r.get("module").and_then(|m| m.as_str());
			let r_mode = r.get("mode").unwrap().as_str().unwrap();
			let r_ty = r.get("type").unwrap().as_str().unwrap();
			let r_name = r.get("name").unwrap().as_str().unwrap();
			r_module == dst_module && r_mode == dst_mode && r_ty == dst_ty && r_name == dst_name
		});
	if let Some(existing_dst_resource) = existing_dst_resource {
		println!("Already has resource: {dst_module:?}.{dst_mode}.{dst_ty}.{dst_name}");
		return Ok(());
	}

	// Write resource
	dst_state["resources"].as_array_mut().unwrap().push(json!({
		"module": dst_module,
		"mode": dst_mode,
		"type": dst_ty,
		"name": dst_name,
		"provider": src_resource["provider"],
		"instances": src_resource["instances"]
	}));

	Ok(())
}

async fn copy_resource_module_prefix(
	ctx: &ProjectContext,
	src_state: &serde_json::Value,
	(src_module_prefix, src_mode, src_ty, src_name): (&str, &str, &str, &str),
	dst_state: &mut serde_json::Value,
	(dst_mode, dst_ty, dst_name): (&str, &str, &str),
) -> Result<()> {
	// Find resource
	let mut src_resource = src_state["resources"]
		.as_array()
		.unwrap()
		.iter()
		.find(|r| {
			let r = r.as_object().unwrap();
			let r_module = r.get("module").and_then(|m| m.as_str());
			let r_mode = r.get("mode").unwrap().as_str().unwrap();
			let r_ty = r.get("type").unwrap().as_str().unwrap();
			let r_name = r.get("name").unwrap().as_str().unwrap();
			r_module.map_or(false, |x| x.starts_with(src_module_prefix))
				&& r_mode == src_mode
				&& r_ty == src_ty
				&& r_name == src_name
		})
		.context(format!(
			"resource not found: {src_module_prefix:?}*.{src_mode}.{src_ty}.{src_name}"
		))?
		.clone();
	let src_module = src_resource["module"].as_str().unwrap().to_string();
	println!("Found resource: {src_module}.{src_mode}.{src_ty}.{src_name}");

	// Remove all dependencies since they may contain invalid references. This
	// will be re-populated when applied.
	for instance in src_resource
		.get_mut("instances")
		.unwrap()
		.as_array_mut()
		.unwrap()
	{
		instance["dependencies"] = json!([]);
	}

	// Check if already has resource
	let existing_dst_resource = dst_state
		.get_mut("resources")
		.unwrap()
		.as_array_mut()
		.unwrap()
		.iter_mut()
		.find(|r| {
			let r = r.as_object().unwrap();
			let r_module = r.get("module").and_then(|m| m.as_str());
			let r_mode = r.get("mode").unwrap().as_str().unwrap();
			let r_ty = r.get("type").unwrap().as_str().unwrap();
			let r_name = r.get("name").unwrap().as_str().unwrap();
			r_module.map_or(false, |x| x == src_module)
				&& r_mode == dst_mode
				&& r_ty == dst_ty
				&& r_name == dst_name
		});
	if let Some(existing_dst_resource) = existing_dst_resource {
		println!("Already has resource: {src_module:?}.{dst_mode}.{dst_ty}.{dst_name}");
		return Ok(());
	}

	// Write resource
	dst_state["resources"].as_array_mut().unwrap().push(json!({
		"module": src_module,
		"mode": dst_mode,
		"type": dst_ty,
		"name": dst_name,
		"provider": src_resource["provider"],
		"instances": src_resource["instances"]
	}));

	Ok(())
}

async fn copy_resource_with_index_to_module_with_index(
	ctx: &ProjectContext,
	src_state: &serde_json::Value,
	(src_module, src_mode, src_ty, src_name): (Option<&str>, &str, &str, &str),
	dst_state: &mut serde_json::Value,
	(dst_module, dst_mode, dst_ty, dst_name): (&str, &str, &str, &str),
) -> Result<()> {
	// Find resource
	let mut src_resource = src_state["resources"]
		.as_array()
		.unwrap()
		.iter()
		.find(|r| {
			let r = r.as_object().unwrap();
			let r_module = r.get("module").map(|m| m.as_str().unwrap());
			let r_mode = r.get("mode").unwrap().as_str().unwrap();
			let r_ty = r.get("type").unwrap().as_str().unwrap();
			let r_name = r.get("name").unwrap().as_str().unwrap();
			r_module == src_module && r_mode == src_mode && r_ty == src_ty && r_name == src_name
		})
		.context(format!(
			"resource not found: {src_module:?}.{src_mode}.{src_ty}.{src_name}"
		))?
		.clone();
	println!("Found resource: {src_module:?}.{src_mode}.{src_ty}.{src_name}");

	// Insert all instances to individual resources
	for mut instance in src_resource
		.get_mut("instances")
		.unwrap()
		.as_array()
		.unwrap()
		.clone()
	{
		let index_key = instance
			.get("index_key")
			.expect("not a resource map")
			.as_str()
			.unwrap()
			.to_string();

		// Clear index key
		instance.as_object_mut().unwrap().remove("index_key");

		// Check if already has resource
		let dst_module_indexed = format!("{dst_module}[\"{index_key}\"]");
		let existing_dst_resource = dst_state
			.get_mut("resources")
			.unwrap()
			.as_array_mut()
			.unwrap()
			.iter_mut()
			.find(|r| {
				let r = r.as_object().unwrap();
				let r_module = r.get("module").unwrap().as_str().unwrap();
				let r_mode = r.get("mode").unwrap().as_str().unwrap();
				let r_ty = r.get("type").unwrap().as_str().unwrap();
				let r_name = r.get("name").unwrap().as_str().unwrap();
				r_module == dst_module_indexed
					&& r_mode == dst_mode
					&& r_ty == dst_ty && r_name == dst_name
			});
		if let Some(existing_dst_resource) = existing_dst_resource {
			// for instance in existing_dst_resource
			// 	.get_mut("instances")
			// 	.unwrap()
			// 	.as_array_mut()
			// 	.unwrap()
			// {
			// 	instance.as_object_mut().unwrap().remove("index_key");
			// }
			println!("Already has resource: {dst_module_indexed}.{dst_mode}.{dst_ty}.{dst_name}");
			continue;
		}

		// Remove all dependencies since they may contain invalid references. This
		// will be re-populated when applied.
		instance["dependencies"] = json!([]);

		// Write resource
		dst_state["resources"].as_array_mut().unwrap().push(json!({
			"module": dst_module_indexed,
			"mode": dst_mode,
			"type": dst_ty,
			"name": dst_name,
			"provider": src_resource["provider"],
			"instances": [instance]
		}));
	}

	Ok(())
}

// async fn import_linode_servers() -> Result<Vec<TfImport>> {
// 	let servers = fetch_linode_servers().await?;

//     // Process servers
//     let mut imports = Vec::new();
//     for server in servers {
//         if !server.tags.contains(ctx.ns_id()) {
//             continue;
//         }

// 		let label = server.label;

//         // TODO: infra/tf/master_cluster/nebula_lighthouse.tf
//         // TODO: infra/tf/master_cluster/salt_master.tf

//         // infra/tf/pools/servers.tf
// 		// TODO: module.servers["test1-lnd-atl-ing-job-01-0"].data.tls_public_key.server
// 		// TODO: module.servers["test1-lnd-atl-ing-job-01-0"].linode_firewall.server[0]
// 		imports.push(Import { plan: "pools".into(),
// 			resource: format!("module.servers[\"{label}\"].linode_instance.server[0]"),
// 			id: server.id.to_string(),
// 		});

// 		let configs = fetch_linode_configs(server.id).await?;
// 		assert!(configs.len() == 1, "wrong config count: {configs:?}");
// 		imports.push(Import { plan: "pools".into(),
// 			resource: format!("module.servers[\"{label}\"].linode_instance_config.server_boot_config[0]"),
// 			id: configs[0].id.to_string(),
// 		});

// 		module.servers["test1-lnd-atl-ing-job-01-0"].linode_instance_disk.server_boot[0]
// 		module.servers["test1-lnd-atl-ing-job-01-0"].linode_instance_disk.server_swap[0]
// 		module.servers["test1-lnd-atl-ing-job-01-0"].linode_sshkey.server
// 		module.servers["test1-lnd-atl-ing-job-01-0"].random_string.server_root_pass[0]
//     }

//     Ok(imports)

// }

// fn linode_client() -> Result<reqwest::Client>{
// 	// Build client
// 	let linode_token = std::env::var("RIVET_LINODE_TOKEN").context("missing RIVET_LINODE_TOKEN")?;
// 	let mut headers = reqwest::header::HeaderMap::new();
// 	headers.insert(
// 		reqwest::header::AUTHORIZATION,
// 		reqwest::header::HeaderValue::from_str(&format!("Bearer {linode_token}"))?,
// 	);
// 	let client = reqwest::Client::builder()
// 		.default_headers(headers)
// 		.build()?;

// 		Ok(client)
// }

// #[derive(Deserialize, Debug)]
// #[allow(unused)]
// struct Instance {
// 	id: i64,
// 	label: String,
// 	tags: Vec<String>,
// }

// #[derive(Deserialize, Debug)]
// struct LinodeInstanceResponse {
// 	data: Vec<Instance>,
// }

// async fn fetch_linode_servers() -> Result<Vec<Instance>> {
// 	let resp = linode_client()
// 		.get("https://api.linode.com/v4/linode/instances")
// 		.send()
// 		.await?;
// 	let servers: LinodeRegionsResponse = resp.json().await?;

// 	Ok(servers.data)
// }

// #[derive(Deserialize, Debug)]
// #[allow(unused)]
// struct InstanceConfig {
// 	id: i64,
// }

// #[derive(Deserialize, Debug)]
// struct LinodeConfigsResponse {
// 	data: Vec<Config>,
// }

// async fn fetch_linode_configs(id: i64) -> Result<Vec<InstanceConfig>> {
// 	let resp = linode_client()
// 		.get(format!("https://api.linode.com/v4/linode/instances/{id}/configs"))
// 		.send()
// 		.await?;
// 	let configs: LinodeConfigsResponse = resp.json().await?;

// 	Ok(configs.data)
// }

// #[derive(Deserialize, Debug)]
// #[allow(unused)]
// struct InstanceDisk {
// 	id: i64,
// 	filesystem: String,
// 	label: String,
// }

// #[derive(Deserialize, Debug)]
// struct LinodeInstanceResponse {
// 	data: Vec<Disk>,
// }

// async fn fetch_linode_disks(id: i64) -> Result<Vec<InstanceDisk>> {
// 	let resp = linode_client()
// 		.get(format!("https://api.linode.com/v4/linode/instances/{id}/disks"))
// 		.send()
// 		.await?;
// 	let disks: LinodeConfigsResponse = resp.json().await?;

// 	Ok(disks.data)
// }
