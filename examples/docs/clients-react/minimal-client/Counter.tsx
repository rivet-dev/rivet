import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./index";

const { useActor } = createRivetKit<typeof registry>({
  endpoint: "https://my-namespace:pk_...@api.rivet.dev",
});

function Counter() {
  const { connection, connStatus } = useActor({ name: "counter", key: ["my-counter"] });

  if (connStatus !== "connected" || !connection) return <div>Connecting...</div>;
  return <button onClick={() => connection.increment(1)}>+</button>;
}
