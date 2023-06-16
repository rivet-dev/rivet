use api_helper::{define_router, util::CorsConfigBuilder};
use hyper::{Body, Request, Response};
use rivet_chat_server::models;

use uuid::Uuid;

mod chat;
mod identities;

pub async fn handle(
	shared_client: chirp_client::SharedClientHandle,
	pools: rivet_pools::Pools,
	cache: rivet_cache::Cache,
	ray_id: uuid::Uuid,
	request: Request<Body>,
) -> Result<Response<Body>, http::Error> {
	let response = Response::builder();

	// Handle route
	Router::handle(shared_client, pools, cache, ray_id, request, response).await
}

define_router! {
	cors: CorsConfigBuilder::public().build(),
	routes: {
		"identities" / Uuid / "thread": {
			GET: identities::get_direct_thread(),
		},
		"threads" / Uuid / "topic": {
			GET: chat::thread_topic(),
		},
		"threads" / Uuid / "live": {
			GET: chat::thread_live(),
		},
		"threads" / Uuid / "history": {
			GET: chat::thread_history(query: chat::GetThreadHistoryQuery),
		},
		"threads" / Uuid / "read": {
			POST: chat::set_thread_read(body: models::SetThreadReadRequest),
		},
		"threads" / Uuid / "typing-status": {
			PUT: chat::set_typing_status(body: models::SetTypingStatusRequest),
		},
		"messages": {
			POST: chat::send_chat_message(
				body: models::SendChatMessageRequest,
				rate_limit: {
					buckets: [
						{ count: 2 },
					],
				},
			),
		},
	}
}
