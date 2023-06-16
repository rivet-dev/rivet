use uuid::Uuid;

/// HASH
pub fn search_user(user_id: Uuid) -> String {
	format!("search:user:{user_id}")
}

/// HASH
pub fn search_team(team_id: Uuid) -> String {
	format!("search:team:{team_id}")
}
