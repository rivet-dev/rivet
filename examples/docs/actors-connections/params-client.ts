import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface ConnParams {
  authToken: string;
}

interface ConnState {
  userId: string;
  role: string;
}

const gameRoom = actor({
  state: {},
  createConnState: (c, params: ConnParams): ConnState => {
    return { userId: "user-123", role: "player" };
  },
  actions: {}
});

const registry = setup({ use: { gameRoom } });
const client = createClient<typeof registry>("http://localhost:6420");

async function getAuthToken(): Promise<string> {
  return "supersekure";
}

const gameRoomHandle = client.gameRoom.getOrCreate(["room-123"], {
  getParams: async () => ({
    authToken: await getAuthToken(),
  })
});
