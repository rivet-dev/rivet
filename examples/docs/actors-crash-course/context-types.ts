import { actor, ActionContextOf } from "rivetkit";

const gameRoom = actor({
	state: { players: [] as string[], score: 0 },
	actions: {
		addPlayer: (c, playerId: string) => {
			validatePlayer(c, playerId);
			c.state.players.push(playerId);
		},
	},
});

// Good: derive context type from actor definition
function validatePlayer(c: ActionContextOf<typeof gameRoom>, playerId: string) {
	if (c.state.players.includes(playerId)) {
		throw new Error("Player already in room");
	}
}

// Bad: don't manually define context types like this
// type MyContext = { state: { players: string[] }; ... };
