use rivet_util as util;

pub const GAME_USER_TOKEN_TTL: i64 = util::duration::days(180);

// Duration since the issue date that we will issue a new token. This should be
// significantly less than `GAME_USER_TOKEN_TTL`.
//
// i.e. this means that if the user does not visit the game for at least
// `GAME_USER_TOKEN_TTL - GAME_USER_TOKEN_REFRESH_AGE`, the token will expire
// next time they visit the game.
pub const GAME_USER_TOKEN_REFRESH_AGE: i64 = util::duration::days(30);
