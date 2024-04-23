use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

#[derive(serde::Serialize)]
struct ScoreRequest {
	images: Vec<ScoreRequestImage>,
}

#[derive(serde::Serialize)]
struct ScoreRequestImage {
	url: String,
	id: usize,
}

#[derive(serde::Deserialize)]
struct ScoreResponse {
	predictions: Vec<ScorePrediction>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ScorePrediction {
	Success {
		url: String,
		score: f32,
	},
	Error {
		url: String,
		error_code: usize,
		error_reason: String,
	},
}

#[operation(name = "nsfw-image-score")]
async fn handle(
	ctx: OperationContext<nsfw::image_score::Request>,
) -> GlobalResult<nsfw::image_score::Response> {
	// NSFW API disabled, return default response
	if util::env::var("RIVET_UPLOAD_NSFW_CHECK_ENABLED").is_err() {
		return Ok(nsfw::image_score::Response {
			scores: ctx
				.image_urls
				.iter()
				.map(|url| nsfw::image_score::response::ImageScore {
					url: url.clone(),
					score: 0.0,
				})
				.collect::<Vec<_>>(),
		});
	}

	let images = ctx
		.image_urls
		.iter()
		.enumerate()
		.map(|(id, url)| ScoreRequestImage {
			url: url.clone(),
			id,
		})
		.collect::<Vec<_>>();

	let res = reqwest::Client::new()
		.post("http://nsfw-api.nsfw-api.svc.cluster.local:21900/batch-classify".to_string())
		.json(&ScoreRequest { images })
		.send()
		.await?
		.error_for_status()?
		.json::<ScoreResponse>()
		.await?;

	let scores = res
		.predictions
		.into_iter()
		.map(|prediction| match prediction {
			ScorePrediction::Success { url, score } => {
				Ok(nsfw::image_score::response::ImageScore { url, score })
			}
			ScorePrediction::Error {
				url,
				error_code,
				error_reason,
			} => Err(err_code!(
				NSFW_IMAGE_REQUEST_FAILED,
				url = url,
				error_code = error_code,
				error_reason = error_reason
			)),
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	let events = scores
		.iter()
		.map(|x| {
			GlobalResult::Ok(analytics::msg::event_create::Event {
				event_id: Some(Uuid::new_v4().into()),
				name: "nsfw.score".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"url": x.url,
					"score": x.score,
				}))?),
				..Default::default()
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;
	msg!([ctx] analytics::msg::event_create() {
		events: events,
	})
	.await?;

	Ok(nsfw::image_score::Response { scores })
}
