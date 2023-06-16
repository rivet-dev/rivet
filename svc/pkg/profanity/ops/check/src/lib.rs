use censor::Censor;
use lazy_static::lazy_static;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

lazy_static! {
	pub static ref PROFANITY_CENSOR: Censor = Censor::Standard + Censor::Sex;
}

#[operation(name = "profanity-check")]
async fn handle(
	ctx: OperationContext<profanity::check::Request>,
) -> GlobalResult<profanity::check::Response> {
	let disable_filter = ctx.test()
		|| std::env::var("RIVET_PROFANITY_FILTER_DISABLE")
			.ok()
			.map_or(true, |x| x == "1");

	let (results, censored_results) = if !disable_filter {
		if ctx.censor {
			(
				Vec::new(),
				ctx.strings
					.iter()
					.map(|s| PROFANITY_CENSOR.censor(s))
					.collect::<Vec<_>>(),
			)
		} else {
			(
				ctx.strings
					.iter()
					.map(|s| PROFANITY_CENSOR.check(s))
					.collect::<Vec<_>>(),
				Vec::new(),
			)
		}
	} else {
		(
			std::iter::repeat(false)
				.take(ctx.strings.len())
				.collect::<Vec<_>>(),
			Vec::new(),
		)
	};

	Ok(profanity::check::Response {
		results,
		censored_results,
	})
}
