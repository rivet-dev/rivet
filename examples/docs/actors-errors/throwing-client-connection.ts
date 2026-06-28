import { actor, setup } from "rivetkit";
import { createClient, ActorError } from "rivetkit/client";

const user = actor({
  state: { username: "" },
  actions: {
    updateUsername: (c, username: string) => {
      if (username.length > 32) throw new Error("Username is too long");
      c.state.username = username;
    }
  }
});

const registry = setup({ use: { user } });
const client = createClient<typeof registry>("http://localhost:6420");
const conn = client.user.getOrCreate([]).connect();

try {
  await conn.updateUsername("extremely_long_username_that_exceeds_the_limit");
} catch (error) {
  if (error instanceof ActorError) {
    console.log(error.message); // "Username is too long"
  }
}
