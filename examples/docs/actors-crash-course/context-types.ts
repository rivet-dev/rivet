import { actor, ActionContextOf } from "rivetkit";

const gameRoom = actor({
	vars: { players: [] as string[], score: 0 },
	actions: {
		addPlayer: (c, playerId: string) => {
			validatePlayer(c, playerId);
			c.vars.players.push(playerId);
		},
	},
});

// Good: derive context type from actor definition
function validatePlayer(c: ActionContextOf<typeof gameRoom>, playerId: string) {
	if (c.vars.players.includes(playerId)) {
		throw new Error("Player already in room");
	}
}

// Bad: don't manually define context types like this
// type MyContext = { vars: { players: string[] }; ... };
