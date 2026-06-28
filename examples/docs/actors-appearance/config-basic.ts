import { actor } from "rivetkit";

const chatRoom = actor({
  options: {
    name: "Chat Room",    // Human-friendly display name
    icon: "comments",     // FontAwesome icon name
  },
  state: { messages: [] },
  actions: {
    // ...
  }
});
