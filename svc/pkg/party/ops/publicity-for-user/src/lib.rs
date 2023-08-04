use proto::backend::{party::party::PublicityLevel, pkg::*};
use rivet_operation::prelude::*;
use std::collections::HashSet;

#[derive(Debug)]
struct Party {
	party_id: Uuid,
	user_ids: Vec<Uuid>,
	publicity: PartyPublicity,
}

#[derive(Debug)]
struct PartyPublicity {
	public: PublicityLevel,
	friends: PublicityLevel,
	teams: PublicityLevel,
}

#[operation(name = "party-publicity-for-user")]
async fn handle(
	ctx: OperationContext<party::publicity_for_user::Request>,
) -> GlobalResult<party::publicity_for_user::Response> {
	// TODO:
	return Ok(party::publicity_for_user::Response {
		parties: Vec::new(),
	});

	let this_user_id = internal_unwrap!(ctx.user_id).as_uuid();

	// Fetch parties
	let (get_res, member_list_res) = tokio::try_join!(
		op!([ctx] party_get {
			party_ids: ctx.party_ids.clone(),
		}),
		op!([ctx] party_member_list {
			party_ids: ctx.party_ids.clone(),
		})
	)?;

	// Aggregate parties
	let parties = get_res
		.parties
		.iter()
		.map(|party| {
			let party_id = internal_unwrap!(party.party_id).as_uuid();

			let publicity = internal_unwrap!(party.publicity).clone();

			let members_party = member_list_res
				.parties
				.iter()
				.find(|x| x.party_id == party.party_id);
			let members_party = internal_unwrap_owned!(members_party, "missing matching members");
			let user_ids = members_party
				.user_ids
				.iter()
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>();

			GlobalResult::Ok(Party {
				party_id,
				user_ids,
				publicity: PartyPublicity {
					public: PublicityLevel::from_i32(publicity.public)
						.unwrap_or(PublicityLevel::None),
					friends: PublicityLevel::from_i32(publicity.friends)
						.unwrap_or(PublicityLevel::None),
					teams: PublicityLevel::from_i32(publicity.teams)
						.unwrap_or(PublicityLevel::None),
				},
			})
		})
		.collect::<GlobalResult<Vec<Party>>>()?;
	tracing::info!(?parties, "fetched parties");

	let user_ids = parties
		.iter()
		// Will always be join no matter the relationship.
		.filter(|p| p.publicity.public != PublicityLevel::Join)
		// Relationship makes no difference.
		.filter(|p| p.publicity.friends != PublicityLevel::None)
		.flat_map(|x| x.user_ids.iter())
		// Validate not querying self
		.filter(|user_id| user_id != &&this_user_id)
		.map(|x| *x)
		.collect::<HashSet<Uuid>>();

	// Fetch follow relationships.
	let user_follows = user_ids
		.iter()
		.cloned()
		.map(|user_id| user_follow::relationship_get::request::User {
			this_user_id: Some(this_user_id.into()),
			other_user_id: Some(user_id.into()),
		})
		.collect::<Vec<_>>();

	// Fetch member relationships.
	let team_members = user_ids
		.into_iter()
		.map(|user_id| team::member_relationship_get::request::User {
			this_user_id: Some(this_user_id.into()),
			other_user_id: Some(user_id.into()),
		})
		.collect::<Vec<_>>();

	let (user_follow_relationships, team_member_relationships) = tokio::try_join!(
		op!([ctx] user_follow_relationship_get {
			users: user_follows.clone(),
		}),
		op!([ctx] team_member_relationship_get {
			users: team_members.clone(),
		}),
	)?;
	tracing::info!(?user_follows, user_follow_relationships = ?user_follow_relationships.users, ?team_members, team_member_relationships = ?team_member_relationships.users, "queried relationships");

	// Calculate party publicity
	let parties = parties
		.iter()
		.map(|party| {
			let party_span = tracing::info_span!("party", party_id = %party.party_id);
			let _enter = party_span.enter();

			// Determine publicity
			let mut publicity = party.publicity.public;
			for &user_id in &party.user_ids {
				let user_span = tracing::info_span!("user", %user_id);
				let _enter = user_span.enter();

				tracing::info!("checking user");

				let user_id_proto = Some(Into::<common::Uuid>::into(user_id));

				// Friends
				for (other_user_id, relationship) in user_follows
					.iter()
					.zip(user_follow_relationships.users.iter())
					.filter(|(x, _)| x.other_user_id == user_id_proto)
				{
					tracing::info!(?other_user_id, ?relationship, "user follow relationship");
					if relationship.is_mutual {
						publicity = max_publicity(publicity, party.publicity.friends);
					}
				}

				// Teams
				for (other_user_id, relationship) in team_members
					.iter()
					.zip(team_member_relationships.users.iter())
					.filter(|(x, _)| x.other_user_id == user_id_proto)
				{
					tracing::info!(?other_user_id, ?relationship, "team member relationship");
					if !relationship.shared_team_ids.is_empty() {
						publicity = max_publicity(publicity, party.publicity.teams);
					}
				}
			}

			party::publicity_for_user::response::Party {
				party_id: Some(party.party_id.into()),
				publicity: publicity as i32,
			}
		})
		.collect();

	Ok(party::publicity_for_user::Response { parties })
}

/// Returns a priority for a given publicity. Higher = takes presence over lower
/// ranks.
#[tracing::instrument]
fn rank_publicity(x: PublicityLevel) -> i32 {
	match x {
		PublicityLevel::None => 0,
		PublicityLevel::View => 1,
		PublicityLevel::Join => 2,
	}
}

/// Returns the publicity with the higher rank.
#[tracing::instrument]
fn max_publicity(a: PublicityLevel, b: PublicityLevel) -> PublicityLevel {
	if rank_publicity(a) > rank_publicity(b) {
		tracing::info!(from = ?a, to = ?b, "upgrading publicity");
		a
	} else {
		b
	}
}
