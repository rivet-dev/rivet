import { actor, setup, UserError } from "rivetkit";
import { createClient, ActorError } from "rivetkit/client";

const user = actor({
  state: { username: "" },
  actions: {
    updateUsername: (c, username: string) => {
      if (username.length < 3) {
        throw new UserError("Username too short", {
          code: "username_too_short",
          metadata: { minLength: 3, actual: username.length },
        });
      }
      c.state.username = username;
    },
  },
});

const registry = setup({ use: { user } });
const client = createClient<typeof registry>("http://localhost:6420");

try {
  await client.user.getOrCreate([]).updateUsername("ab");
} catch (error) {
  if (error instanceof ActorError) {
    console.log(error.code);     // "username_too_short"
    console.log(error.metadata); // { minLength: 3, actual: 2 }
  }
}
