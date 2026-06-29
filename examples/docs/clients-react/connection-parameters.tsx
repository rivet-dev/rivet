import { createRivetKit } from "@rivetkit/react";

const { useActor } = createRivetKit();

function Chat() {
  const chat = useActor({
    name: "chatRoom",
    key: ["general"],
    params: { authToken: "jwt-token-here" },
  });

  return <div>{chat.connStatus}</div>;
}
