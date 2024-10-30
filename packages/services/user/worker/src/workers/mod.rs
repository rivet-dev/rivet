mod admin_set;
mod create;
mod delete;
mod event_team_member_remove;
mod event_user_update;
mod game_update;
mod profile_set;
mod updated_user_update;

chirp_worker::workers![
	admin_set,
	create,
	delete,
	event_team_member_remove,
	event_user_update,
	game_update,
	profile_set,
	updated_user_update,
];
