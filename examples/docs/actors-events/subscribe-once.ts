import { actor, event, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const gameRoom = actor({
  state: { started: false },
  events: {
    gameStarted: event<[]>()
  },
  actions: {
    startGame: (c) => {
      c.state.started = true;
      c.broadcast('gameStarted');
    }
  }
});

const registry = setup({ use: { gameRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

function showGameInterface() {
  console.log("Showing game interface");
}

const gameRoomHandle = client.gameRoom.getOrCreate(["room-456"]);
const connection = gameRoomHandle.connect();

// Listen for game start (only once)
connection.once('gameStarted', () => {
  console.log('Game has started!');
  showGameInterface();
});
