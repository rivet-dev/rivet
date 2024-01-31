use proto::backend::pkg::*;
use rivet_operation::prelude::*;

lazy_static::lazy_static! {
	static ref QUERY_SPLIT_RE: regex::Regex = regex::Regex::new(r"^(?P<query>.*?)#(?P<number>\d{1,4})$$").unwrap();
}

#[derive(sqlx::FromRow)]
struct User {
	user_id: Uuid,
	join_ts: i64,
}

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("extract regex capture")]
	ExtractRegexCapture,
}

#[operation(name = "user-search")]
async fn handle(
	ctx: OperationContext<user::search::Request>,
) -> GlobalResult<user::search::Response> {
	let _crdb = ctx.crdb().await?;
	let limit = ctx.limit;

	ensure!(limit != 0, "limit too low");
	ensure!(limit <= 32, "limit too high");

	// Parse name and account number bounds from query
	let (query, lower, upper): (String, i64, i64) =
		if let Some(captures) = QUERY_SPLIT_RE.captures(&ctx.query) {
			let query = captures
				.name("query")
				.ok_or(Error::ExtractRegexCapture)?
				.as_str()
				.to_owned();
			let number_raw = captures
				.name("number")
				.ok_or(Error::ExtractRegexCapture)?
				.as_str();

			let digit_count = number_raw.len();
			let number = number_raw.parse::<u16>()?; // Must be unsigned so we don't parse a negative sign

			let shift = 10i64.pow(4u32 - digit_count as u32);
			let lower = (number as i64) * shift;
			let upper = lower + shift;

			(query, lower, upper)
		} else {
			(ctx.query.to_owned(), 0, 10000)
		};

	let res = sql_fetch_all!(
		[ctx, User]
		"
		SELECT user_id, join_ts FROM db_user.users@search_index
		WHERE
			display_name % $1 AND
			account_number >= $2 AND
			account_number < $3 AND
			is_searchable = TRUE AND
			join_ts < $4
			ORDER BY join_ts DESC
			LIMIT $5
		",
		query,
		lower,
		upper,
		ctx.anchor.unwrap_or_else(util::timestamp::now),
		limit as i64,
	)
	.await?;

	let anchor = res.last().map(|user| user.join_ts);

	Ok(user::search::Response {
		user_ids: res
			.into_iter()
			.map(|user| user.user_id.into())
			.collect::<Vec<_>>(),
		anchor,
	})
}
