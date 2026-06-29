import { createRivetKit } from "@rivetkit/react";

const { useActor } = createRivetKit();

function CounterStatus() {
  const actor = useActor({ name: "counter", key: ["my-counter"] });

  if (actor.connStatus === "connected") {
    console.log("connected");
  }

  if (actor.error) {
    console.error(actor.error);
  }

  return null;
}
