import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

interface ConnParams {
  authToken: string;
}

const gameRoom = actor({
  onBeforeConnect: (c, params: ConnParams) => {},
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
