import { createRivetKit } from "@rivetkit/react";

const { useActor } = createRivetKit();

function Chat() {
  const chat = useActor({ name: "chatRoom", key: ["general"] });

  chat.useEvent("message", (msg) => {
    console.log("message:", msg);
  });

  return null;
}
