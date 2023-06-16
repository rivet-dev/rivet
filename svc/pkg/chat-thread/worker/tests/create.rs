use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

struct ThreadEntry {
	thread_id: Option<Uuid>,
	kind: backend::chat::topic::Kind,
}

#[derive(sqlx::FromRow)]
struct Thread {
	thread_id: Uuid,

	team_team_id: Option<Uuid>,
	party_party_id: Option<Uuid>,
	direct_user_a_id: Option<Uuid>,
	direct_user_b_id: Option<Uuid>,
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let mut threads = vec![
		ThreadEntry {
			thread_id: None,
			kind: backend::chat::topic::Kind::Team(backend::chat::topic::Team {
				team_id: Some(Uuid::new_v4().into()),
			}),
		},
		ThreadEntry {
			thread_id: None,
			kind: backend::chat::topic::Kind::Party(backend::chat::topic::Party {
				party_id: Some(Uuid::new_v4().into()),
			}),
		},
		ThreadEntry {
			thread_id: None,
			kind: backend::chat::topic::Kind::Direct(backend::chat::topic::Direct {
				user_a_id: Some(Uuid::new_v4().into()),
				user_b_id: Some(Uuid::new_v4().into()),
			}),
		},
	];

	for thread in &mut threads {
		let request_id = Uuid::new_v4();
		let create_res =
			msg!([ctx] chat_thread::msg::create(request_id) -> chat_thread::msg::create_complete {
				request_id: Some(request_id.into()),
				topic: Some(backend::chat::Topic {
					kind: Some(thread.kind.clone()),
				}),
				override_create_ts: None,
			})
			.await
			.unwrap();
		thread.thread_id = Some(create_res.thread_id.unwrap().as_uuid());
	}

	let thread_ids = threads.iter().flat_map(|t| t.thread_id).collect::<Vec<_>>();
	let crdb = ctx.crdb("db-chat").await.unwrap();
	let crdb_threads = sqlx::query_as::<_, Thread>(indoc!(
		"
		SELECT
			thread_id,
			team_team_id,
			party_party_id,
			direct_user_a_id,
			direct_user_b_id
		FROM threads
		WHERE thread_id = ANY($1)
		"
	))
	.bind(thread_ids)
	.fetch_all(&crdb)
	.await
	.unwrap();
	assert_eq!(threads.len(), crdb_threads.len());

	for crdb_thread in crdb_threads {
		let thread = threads
			.iter()
			.find(|t| t.thread_id.unwrap() == crdb_thread.thread_id)
			.expect("missing matching thread");

		match &thread.kind {
			backend::chat::topic::Kind::Team(team) => {
				assert_eq!(None, crdb_thread.party_party_id);
				assert_eq!(None, crdb_thread.direct_user_a_id);
				assert_eq!(None, crdb_thread.direct_user_b_id);
				assert_eq!(
					team.team_id.unwrap().as_uuid(),
					crdb_thread.team_team_id.unwrap()
				);
			}
			backend::chat::topic::Kind::Party(party) => {
				assert_eq!(None, crdb_thread.team_team_id);
				assert_eq!(None, crdb_thread.direct_user_a_id);
				assert_eq!(None, crdb_thread.direct_user_b_id);
				assert_eq!(
					party.party_id.unwrap().as_uuid(),
					crdb_thread.party_party_id.unwrap()
				);
			}
			backend::chat::topic::Kind::Direct(direct) => {
				assert_eq!(None, crdb_thread.team_team_id);
				assert_eq!(None, crdb_thread.party_party_id);

				let (user_a_id, user_b_id) = util::sort::id_pair(
					direct.user_a_id.unwrap().as_uuid(),
					direct.user_b_id.unwrap().as_uuid(),
				);
				assert_eq!(user_a_id, crdb_thread.direct_user_a_id.unwrap());
				assert_eq!(user_b_id, crdb_thread.direct_user_b_id.unwrap());
			}
		}
	}
}
