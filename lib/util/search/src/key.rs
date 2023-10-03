use uuid::Uuid;

/// HASH
pub fn search_user(user_id: Uuid) -> String {
	format!("{{global}}:search:user:{user_id}")
}

/// HASH
pub fn search_team(team_id: Uuid) -> String {
	format!("{{global}}:search:team:{team_id}")
}
