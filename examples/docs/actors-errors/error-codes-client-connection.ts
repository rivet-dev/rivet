import { actor, setup } from "rivetkit";
import { createClient, ActorError } from "rivetkit/client";

const user = actor({
  state: { username: "" },
  actions: {
    updateUsername: (c, username: string) => { c.state.username = username; }
  }
});

const registry = setup({ use: { user } });
const client = createClient<typeof registry>("http://localhost:6420");
const conn = client.user.getOrCreate([]).connect();

try {
  await conn.updateUsername("ab");
} catch (error) {
  if (error instanceof ActorError) {
    if (error.code === "username_too_short") {
      console.log("Please choose a longer username");
    } else if (error.code === "username_too_long") {
      console.log("Please choose a shorter username");
    }
  }
}
