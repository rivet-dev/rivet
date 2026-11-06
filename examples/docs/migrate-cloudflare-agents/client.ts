import { createClient } from "rivetkit/client";
import type { registry } from "./state-sync";

const client = createClient<typeof registry>("http://localhost:6420");

// Like AgentClient: connect to one instance by name and key
const game = client.game.getOrCreate(["game-123"]).connect();

function render(score: number) {
	console.log("score", score);
}

// Like onStateUpdate: follow changes the actor broadcasts
game.on("stateChanged", (state) => render(state.score));

// Load the current state once, like the initial state useAgent receives
render((await game.getState()).score);

// Like agent.call() or agent.stub: change state through an action
await game.addScore(10);
