import { ActorError } from "rivetkit/client";
import { createRivetKit } from "@rivetkit/react";

const { useActor } = createRivetKit();

function Profile() {
  const actor = useActor({ name: "user", key: ["user-123"] });

  const updateUsername = async () => {
    try {
      await actor.connection?.updateUsername("ab");
    } catch (error) {
      if (error instanceof ActorError) {
        console.log(error.code, error.metadata);
      }
    }
  };

  return <button onClick={updateUsername}>Update</button>;
}
