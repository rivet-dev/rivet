use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use std::collections::HashMap;

// Use specific utilities to avoid ambiguity
use ::util::check;
use ::util::safe_slice;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Uuid,
	pub name_id: String,
	pub hostname: String,
	pub path: String,
	pub route_subpaths: bool,
	pub strip_prefix: bool,
	pub actors_selector_tags: HashMap<String, String>,
	// Currently only supports actors routes (route_type = 0)
}

#[derive(Debug)]
pub struct Output {
	pub route_id: Uuid,
	pub created: bool, // True if new route was created, false if updated
}

#[operation]
pub async fn upsert(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let domain_job = ctx.config().server()?.rivet.domain_job_for_routes()?;

	// Validate all input fields
	validate_name_id(&input.name_id)?;
	validate_hostname(domain_job, &input.hostname)?;
	validate_path(&input.path)?;
	validate_actors_selector_tags(&input.actors_selector_tags)?;

	let now = ctx.ts();

	// Create the route target (Actors type)
	let target = crate::types::RouteTarget::Actors {
		selector_tags: input.actors_selector_tags.clone(),
	};

	// Serialize the actors selector tags for database storage
	let actors_selector_tags_json = util::serde::Raw::new(&input.actors_selector_tags)?;

	// Use transaction to either create or update the route
	let (route_id, created) = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let input_namespace_id = input.namespace_id;
		let input_name_id = input.name_id.clone();
		let input_hostname = input.hostname.clone();
		let input_path = input.path.clone();
		let input_route_subpaths = input.route_subpaths;
		let input_strip_prefix = input.strip_prefix;
		let target = target.clone();
		let actors_selector_tags_json = actors_selector_tags_json.clone();
		let now = now;

		async move {
			// First check if any other namespace is already using this hostname
			let hostname_conflict = sql_fetch_optional!(
				[ctx, (Uuid,), @tx tx]
				"
				SELECT route_id FROM db_route.routes 
				WHERE hostname = $1 AND namespace_id != $2 AND delete_ts IS NULL
				FOR UPDATE
				",
				&input_hostname,
				input_namespace_id
			)
			.await?;

			// If we found a conflicting route, return an error
			if hostname_conflict.is_some() {
				return Err(err_code!(
					ROUTE_HOSTNAME_ALREADY_EXISTS,
					msg = format!(
						"hostname '{}' is already in use by another environment",
						input_hostname
					)
				));
			}

			// Check if a non-deleted route with this namespace_id and name_id already exists
			let route_id = sql_fetch_optional!(
				[ctx, (Uuid,), @tx tx]
				"
				SELECT route_id FROM db_route.routes 
				WHERE namespace_id = $1 AND name_id = $2 AND delete_ts IS NULL
				FOR UPDATE
				",
				input_namespace_id,
				&input_name_id
			)
			.await?
			.map(|(id,)| id);

			if let Some(existing_id) = route_id {
				// If active route exists, update it
				sql_execute!(
					[ctx, @tx tx]
					"
					UPDATE db_route.routes
					SET hostname = $1, path = $2, route_subpaths = $3, strip_prefix = $4, 
					    route_type = $5, actors_selector_tags = $6, update_ts = $7
					WHERE route_id = $8 AND namespace_id = $9 AND delete_ts IS NULL
					",
					&input_hostname,
					&input_path,
					input_route_subpaths,
					input_strip_prefix,
					0, // Actors type
					actors_selector_tags_json,
					now,
					existing_id,
					input_namespace_id
				)
				.await?;

				Ok((existing_id, false))
			} else {
				// Create a new route
				let new_route_id = Uuid::new_v4();

				sql_execute!(
					[ctx, @tx tx]
					"
					INSERT INTO db_route.routes (
						route_id, namespace_id, name_id, hostname, path, route_subpaths, strip_prefix,
						route_type, actors_selector_tags, create_ts, update_ts, delete_ts
					)
					VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULL)
					",
					new_route_id,
					input_namespace_id,
					&input_name_id,
					&input_hostname,
					&input_path,
					input_route_subpaths,
					input_strip_prefix,
					0, // Actors type
					actors_selector_tags_json,
					now,
					now
				)
				.await?;

				Ok((new_route_id, true))
			}
		}
		.boxed()
	})
	.await?;

	Ok(Output { route_id, created })
}

/// Validates the name_id for a route
fn validate_name_id(name_id: &str) -> GlobalResult<()> {
	ensure_with!(
		check::ident(name_id),
		ROUTE_INVALID_NAME_ID,
		msg = "name_id must be lowercase alphanumeric or dashes without repeating double dashes"
	);

	Ok(())
}

/// Validates the hostname for a route
fn validate_hostname(domain_job: &str, hostname: &str) -> GlobalResult<()> {
	let (subdomain, domain) = hostname.split_once('.').ok_or_else(|| {
		err_code!(
			ROUTE_INVALID_HOSTNAME,
			msg = "hostname must be in format {{xxxx}}.{domain_job}"
		)
	})?;

	ensure_with!(
		subdomain.len() >= 4,
		ROUTE_INVALID_HOSTNAME,
		msg = "hostname subdomain must be at least 4 characters"
	);

	ensure_with!(
		subdomain.len() <= 63,
		ROUTE_INVALID_HOSTNAME,
		msg = "hostname subdomain must be at most 63 characters (DNS limitation)"
	);

	ensure_with!(
		domain == domain_job,
		ROUTE_INVALID_HOSTNAME,
		msg = format!("hostname domain must be {domain_job}")
	);

	// Check that subdomain doesn't start or end with a hyphen (DNS requirement)
	ensure_with!(
		!subdomain.starts_with('-') && !subdomain.ends_with('-'),
		ROUTE_INVALID_HOSTNAME,
		msg = "hostname subdomain must not start or end with a hyphen"
	);

	// Check for consecutive hyphens
	ensure_with!(
		!subdomain.contains("--"),
		ROUTE_INVALID_HOSTNAME,
		msg = "hostname subdomain must not contain consecutive hyphens"
	);

	ensure_with!(
		check::ident_lenient(subdomain),
		ROUTE_INVALID_HOSTNAME,
		msg = "hostname subdomain must contain only alphanumeric characters and hyphens"
	);

	// Ensure the entire hostname is not too long (DNS limitation)
	ensure_with!(
		hostname.len() <= 253,
		ROUTE_INVALID_HOSTNAME,
		msg = "hostname must be at most 253 characters (DNS limitation)"
	);

	Ok(())
}

/// Validates the path for a route
fn validate_path(path: &str) -> GlobalResult<()> {
	// The path should always be "" for root or start with / for others
	if path != "" {
		ensure_with!(
			path.starts_with('/'),
			ROUTE_INVALID_PATH,
			msg = "path must be empty string for root or start with / for all others"
		);

		ensure_with!(
			!path.ends_with('/'),
			ROUTE_INVALID_PATH,
			msg = "path must not end with /"
		);
	}

	// Check for consecutive slashes
	ensure_with!(
		!path.contains("//"),
		ROUTE_INVALID_PATH,
		msg = "path must not contain consecutive slashes"
	);

	let path_components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

	ensure_with!(
		path_components.len() <= crate::utils::MAX_PATH_COMPONENTS,
		ROUTE_INVALID_PATH,
		msg = format!(
			"path can have at most {} components",
			crate::utils::MAX_PATH_COMPONENTS
		)
	);

	ensure_with!(
		path.len() <= 256,
		ROUTE_INVALID_PATH,
		msg = "path must be at most 256 characters"
	);

	// Validate each path component
	for component in &path_components {
		// Check for empty components
		ensure_with!(
			!component.is_empty(),
			ROUTE_INVALID_PATH,
			msg = "path must not contain empty components"
		);

		// Check for valid characters
		ensure_with!(
			component.chars().all(|c| match c {
				'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => true,
				_ => false,
			}),
			ROUTE_INVALID_PATH,
			msg = format!("path component '{}' contains invalid characters", component)
		);

		// Check component length
		ensure_with!(
			component.len() <= 64,
			ROUTE_INVALID_PATH,
			msg = format!(
				"path component '{}' is too long (max 64 characters)",
				component
			)
		);

		// Check that components don't start or end with a dot
		ensure_with!(
			!component.starts_with('.') && !component.ends_with('.'),
			ROUTE_INVALID_PATH,
			msg = format!(
				"path component '{}' must not start or end with a dot",
				component
			)
		);

		// Check for consecutive dots
		ensure_with!(
			!component.contains(".."),
			ROUTE_INVALID_PATH,
			msg = format!(
				"path component '{}' must not contain consecutive dots",
				component
			)
		);
	}

	Ok(())
}

/// Validates the actors_selector_tags for a route
fn validate_actors_selector_tags(selector_tags: &HashMap<String, String>) -> GlobalResult<()> {
	ensure_with!(
		selector_tags.len() >= 1 && selector_tags.len() <= 8,
		ROUTE_INVALID_SELECTOR_TAGS,
		msg = "actors_selector_tags can have at least 1 entry and at most 8 entries"
	);

	for (key, value) in selector_tags {
		ensure_with!(
			key.len() <= 32,
			ROUTE_INVALID_SELECTOR_TAGS,
			msg = format!(
				"selector tag key '{}' is too large (max 32 bytes)",
				safe_slice(key, 0, 32)
			)
		);

		ensure_with!(
			!value.is_empty(),
			ROUTE_INVALID_SELECTOR_TAGS,
			msg = format!("selector tag value for key '{}' cannot be empty", key)
		);

		ensure_with!(
			value.len() <= 1024,
			ROUTE_INVALID_SELECTOR_TAGS,
			msg = format!(
				"selector tag value for key '{}' is too large (max 1024 bytes)",
				key
			)
		);
	}

	Ok(())
}
