import { actor, event } from "rivetkit";
import { DEFAULT_RATING } from "./config.ts";

export const rankedPlayer = actor({
	options: { name: "Ranked - Player", icon: "ranking-star" },
	events: {
		stateUpdate: event<PlayerSnapshot>(),
	},
	state: {
		username: "",
		rating: DEFAULT_RATING,
		wins: 0,
		losses: 0,
	},
	actions: {
		initialize: (c, input: { username: string }) => {
			if (!c.state.username) {
				c.state.username = input.username;
			}
		},
		getProfile: (c): PlayerSnapshot => buildSnapshot(c.state),
		getRating: (c): number => c.state.rating,
		applyMatchResult: (c, input: { won: boolean; newRating: number }) => {
			c.state.rating = input.newRating;
			if (input.won) {
				c.state.wins += 1;
			} else {
				c.state.losses += 1;
			}
			c.broadcast("stateUpdate", buildSnapshot(c.state));
		},
	},
});

export interface PlayerSnapshot {
	username: string;
	rating: number;
	wins: number;
	losses: number;
}

function buildSnapshot(state: { username: string; rating: number; wins: number; losses: number }): PlayerSnapshot {
	return {
		username: state.username,
		rating: state.rating,
		wins: state.wins,
		losses: state.losses,
	};
}
