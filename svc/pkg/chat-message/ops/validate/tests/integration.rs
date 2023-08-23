use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn valid(ctx: TestCtx) {
	let sender_user_id = Uuid::new_v4();
	let team_id = Uuid::new_v4();
	let party_id = Uuid::new_v4();
	let invite_id = Uuid::new_v4();

	// TODO: make a real invite token since this is an error
	// op!([ctx] chat_message_validate {
	// 		message: Some(chat_message_validate::request::Message {
	// 			topic: Some(backend::chat::Topic {
	// 				kind: Some(backend::chat::topic::Kind::Team(backend::chat::topic::Team {
	// 					team_id: Some(team_id.into()),
	// 				}))
	// 			}),
	// 			body: Some(backend::chat::MessageBody {
	// 			kind: Some(backend::chat::message_body::Kind::PartyInvite(
	// 				backend::chat::message_body::PartyInvite {
	// 					sender_user_id: Some(sender_user_id.into()),
	// 					party_id: Some(party_id.into()),
	// 					invite_id: Some(invite_id.into()),
	// 					invite_token: "".to_owned(),
	// 				},
	// 			)),
	// 		}),
	// 	})
	// })
	// .await
	// .unwrap();
}

// #[worker_test]
// async fn invalid(ctx: TestCtx) {
// 	let sender_user_id = Uuid::new_v4();
// 	let party_id = Uuid::new_v4();
// 	let invite_id = Uuid::new_v4();

// 	op!([ctx] chat_message_validate {
// 			message: Some(chat_message::validate::request::Message {
// 				topic: Some(backend::chat::Topic {
// 					kind: Some(backend::chat::topic::Kind::Party(backend::chat::topic::Party {
// 						party_id: Some(party_id.into()),
// 					}))
// 				}),
// 				body: Some(backend::chat::MessageBody {
// 				kind: Some(backend::chat::message_body::Kind::PartyInvite(
// 					backend::chat::message_body::PartyInvite {
// 						sender_user_id: Some(sender_user_id.into()),
// 						party_id: Some(party_id.into()),
// 						invite_id: Some(invite_id.into()),
// 						invite_token: "".to_owned(),
// 					},
// 				)),
// 			}),
// 		})
// 	})
// 	.await
// 	.unwrap_err();
// }
