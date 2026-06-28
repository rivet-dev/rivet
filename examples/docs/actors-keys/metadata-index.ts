import { actor, setup } from "rivetkit";

const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: {
    getRoomName: (c) => {
      // Access the key from metadata
      const key = c.key;
      return key[1]; // Get "general" from ["room", "general"]
    }
  }
});

export const registry = setup({
  use: { chatRoom }
});
