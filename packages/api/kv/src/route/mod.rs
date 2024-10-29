use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_api::models;

mod batch_operations;
mod operations;

define_router! {
	cors: |config| CorsConfigBuilder::public().build(),
	routes: {
		"entries": {
			GET: operations::get(
				query: operations::SingleQuery,
				rate_limit: {
					buckets: [
						{ count: 1_000_000 },
					],
				},
			),
			PUT: operations::put(
				body: models::KvPutRequest,
				rate_limit: {
					buckets: [
						{ count: 100_000 },
					],
				},
			),
			DELETE: operations::delete(
				query: operations::SingleQuery,
				rate_limit: {
					buckets: [
						{ count: 100_000 },
					],
				},
			),
		},
		"entries" / "list": {
			GET: operations::list(query: operations::ListQuery),
		},
		"entries" / "batch": {
			GET: batch_operations::get_batch(
				query: batch_operations::BatchQuery,
				rate_limit: {
					buckets: [
						{ count: 1_000_000 },
					],
				},
			),
			PUT: batch_operations::put_batch(
				body: models::KvPutBatchRequest,
				rate_limit: {
					buckets: [
						{ count: 100_000 },
					],
				},
			),
			DELETE: batch_operations::delete_batch(
				query: batch_operations::BatchQuery,
				rate_limit: {
					buckets: [
						{ count: 100_000 },
					],
				},
			),
		},
	},
}
