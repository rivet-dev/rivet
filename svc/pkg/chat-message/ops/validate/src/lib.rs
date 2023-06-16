use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "chat-message-validate")]
async fn handle(
	ctx: OperationContext<chat_message::validate::Request>,
) -> GlobalResult<chat_message::validate::Response> {
	let message = internal_unwrap!(ctx.message);
	let topic = internal_unwrap!(message.topic);
	let topic_kind = internal_unwrap!(topic.kind);
	let body = internal_unwrap!(message.body);
	let body_kind = internal_unwrap!(body.kind);

	tokio::try_join!(
		validate(&ctx, body_kind, topic_kind),
		// Validate message body
		op!([ctx] chat_message_body_validate {
			body: Some(body.clone()),
		}),
	)?;

	Ok(chat_message::validate::Response {})
}

// TODO: Validate that team join messages only go to team topics, etc
async fn validate(
	ctx: &OperationContext<chat_message::validate::Request>,
	body_kind: &backend::chat::message_body::Kind,
	topic_kind: &backend::chat::topic::Kind,
) -> GlobalResult<()> {
	use backend::chat::{self, message_body::*};

	match (body_kind, topic_kind) {
		(Kind::PartyJoinRequest(PartyJoinRequest { .. }), chat::topic::Kind::Direct(_))
		| (Kind::PartyJoinRequest(PartyJoinRequest { .. }), chat::topic::Kind::Team(_))
		| (Kind::PartyInvite(PartyInvite { .. }), chat::topic::Kind::Party(_)) => {
			panic_with!(CHAT_INVALID_TOPIC);
		}
		// Check if sender of join request can see the party and that the party exists
		(
			Kind::PartyJoinRequest(PartyJoinRequest { sender_user_id }),
			chat::topic::Kind::Party(party),
		) => {
			let party_id = internal_unwrap!(party.party_id);

			let publicity_res = op!([ctx] party_publicity_for_user {
				user_id: *sender_user_id,
				party_ids: vec![*party_id],
			})
			.await?;
			let party = unwrap_with_owned!(publicity_res.parties.first(), PARTY_PARTY_NOT_FOUND);
			let publicity = internal_unwrap_owned!(
				backend::party::party::PublicityLevel::from_i32(party.publicity)
			);

			assert_ne_with!(
				publicity,
				backend::party::party::PublicityLevel::None,
				PARTY_PARTY_NOT_FOUND
			);
		}
		_ => {}
	}

	Ok(())
}
