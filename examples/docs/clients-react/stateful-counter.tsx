import { createRivetKit } from "@rivetkit/react";

const { useActor } = createRivetKit();

function Counter() {
  const counter = useActor({ name: "counter", key: ["my-counter"] });

  const increment = async () => {
    await counter.connection?.increment(1);
  };

  return <button onClick={increment}>+</button>;
}
