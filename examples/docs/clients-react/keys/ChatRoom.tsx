import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./index";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

function ChatRoom() {
  const room = useActor({ name: "chatRoom", key: ["org-acme", "general"] });
  return <div>{room.connStatus}</div>;
}
