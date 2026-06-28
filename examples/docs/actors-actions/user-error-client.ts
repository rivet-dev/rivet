import { actor, setup, UserError } from "rivetkit";
import { ActorError, createClient } from "rivetkit/client";

// Define the user actor
const user = actor({
  state: { username: "" },
  actions: {
    updateUsername: (c, username: string) => {
      if (username.length > 32) {
        throw new UserError("Username is too long", {
          code: "username_too_long",
          metadata: { maxLength: 32 }
        });
      }
      c.state.username = username;
    }
  }
});

const registry = setup({ use: { user } });
const client = createClient<typeof registry>("http://localhost:6420");
const userActor = await client.user.getOrCreate();

try {
  await userActor.updateUsername("extremely_long_username_that_exceeds_limit");
} catch (error) {
  if (error instanceof ActorError) {
    console.log("Message", error.message); // "Username is too long"
    console.log("Code", error.code); // "username_too_long"
    console.log("Metadata", error.metadata); // { maxLength: 32 }
  }
}
