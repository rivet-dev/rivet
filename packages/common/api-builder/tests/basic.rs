use axum_test::TestServer;
use rivet_api_builder::{create_router, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct SinglePathParam {
	id: String,
}

#[derive(Deserialize)]
struct MultiplePathParams {
	namespace_id: String,
	actor_id: String,
}

#[derive(Deserialize)]
struct QueryParam {
	name: Option<String>,
	limit: Option<u32>,
}

#[derive(Deserialize, Serialize)]
struct RequestBody {
	content: String,
}

#[derive(Serialize, Deserialize)]
struct Response {
	message: String,
}

// GET with single path param and query
async fn handle_get_with_query(
	_ctx: ApiCtx,
	path: SinglePathParam,
	query: QueryParam,
) -> Result<Response> {
	Ok(Response {
		message: format!(
			"GET id={} name={:?} limit={:?}",
			path.id, query.name, query.limit
		),
	})
}

// GET with no path params
async fn handle_get_no_params(_ctx: ApiCtx, _path: (), query: QueryParam) -> Result<Response> {
	Ok(Response {
		message: format!(
			"GET no params, query: name={:?} limit={:?}",
			query.name, query.limit
		),
	})
}

// GET with multiple path params
async fn handle_get_multiple(
	_ctx: ApiCtx,
	path: MultiplePathParams,
	_query: (),
) -> Result<Response> {
	Ok(Response {
		message: format!(
			"GET namespace={} actor={}",
			path.namespace_id, path.actor_id
		),
	})
}

// POST
async fn handle_post(
	_ctx: ApiCtx,
	path: SinglePathParam,
	query: QueryParam,
	body: RequestBody,
) -> Result<Response> {
	Ok(Response {
		message: format!(
			"POST id={} name={:?} content={}",
			path.id, query.name, body.content
		),
	})
}

// PUT
async fn handle_put(
	_ctx: ApiCtx,
	path: SinglePathParam,
	_query: (),
	body: RequestBody,
) -> Result<Response> {
	Ok(Response {
		message: format!("PUT id={} content={}", path.id, body.content),
	})
}

// PATCH
async fn handle_patch(
	_ctx: ApiCtx,
	path: SinglePathParam,
	_query: (),
	body: RequestBody,
) -> Result<Response> {
	Ok(Response {
		message: format!("PATCH id={} content={}", path.id, body.content),
	})
}

// DELETE
async fn handle_delete(_ctx: ApiCtx, path: SinglePathParam, _query: ()) -> Result<Response> {
	Ok(Response {
		message: format!("DELETE id={}", path.id),
	})
}

#[tokio::test]
async fn test_api_router() {
	let config = rivet_config::Config::from_root(rivet_config::config::Root::default());
	let pools = rivet_pools::Pools::new(config.clone())
		.await
		.expect("Failed to create test pools");

	// Create router using the create_router function
	let app = create_router("test", config, pools, |router| {
		router
			// GET variations
			.route("/items/{id}", get(handle_get_with_query)) // Single path param with query
			.route("/items", get(handle_get_no_params)) // No path params
			.route(
				"/namespaces/{namespace_id}/actors/{actor_id}",
				get(handle_get_multiple),
			) // Multiple path params
			// Other HTTP methods
			.route("/items/{id}", post(handle_post))
			.route("/items/{id}", put(handle_put))
			.route("/items/{id}", patch(handle_patch))
			.route("/items/{id}", delete(handle_delete))
	})
	.await
	.expect("Failed to create router");

	// Create test server
	let server = TestServer::new(app).unwrap();

	// Test GET with single path param and query
	let res = server
		.get("/items/123")
		.add_query_param("name", "test")
		.add_query_param("limit", "10")
		.await;
	res.assert_status_ok();
	let response: Response = res.json();
	assert_eq!(
		response.message,
		"GET id=123 name=Some(\"test\") limit=Some(10)"
	);

	// Test GET with no path params
	let res = server.get("/items").add_query_param("name", "test2").await;
	res.assert_status_ok();
	let response: Response = res.json();
	assert_eq!(
		response.message,
		"GET no params, query: name=Some(\"test2\") limit=None"
	);

	// Test GET with multiple path params
	let res = server.get("/namespaces/ns123/actors/actor456").await;
	res.assert_status_ok();
	let response: Response = res.json();
	assert_eq!(response.message, "GET namespace=ns123 actor=actor456");

	// Test POST
	let res = server
		.post("/items/456")
		.add_query_param("name", "posttest")
		.json(&RequestBody {
			content: "post content".to_string(),
		})
		.await;
	res.assert_status_ok();
	let response: Response = res.json();
	assert_eq!(
		response.message,
		"POST id=456 name=Some(\"posttest\") content=post content"
	);

	// Test PUT
	let res = server
		.put("/items/789")
		.json(&RequestBody {
			content: "put content".to_string(),
		})
		.await;
	res.assert_status_ok();
	let response: Response = res.json();
	assert_eq!(response.message, "PUT id=789 content=put content");

	// Test PATCH
	let res = server
		.patch("/items/999")
		.json(&RequestBody {
			content: "patch content".to_string(),
		})
		.await;
	res.assert_status_ok();
	let response: Response = res.json();
	assert_eq!(response.message, "PATCH id=999 content=patch content");

	// Test DELETE
	let res = server.delete("/items/111").await;
	res.assert_status_ok();
	let response: Response = res.json();
	assert_eq!(response.message, "DELETE id=111");
}
