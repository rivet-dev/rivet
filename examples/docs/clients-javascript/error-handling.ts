import { ActorError } from "rivetkit/client";
import { createClient } from "rivetkit/client";

const client = createClient();

try {
  await client.user.getOrCreate(["user-123"]).updateUsername("ab");
} catch (error) {
  if (error instanceof ActorError) {
    console.log(error.code, error.metadata);
  }
}
