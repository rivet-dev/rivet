import { actor, setup } from "rivetkit";

export const chatRoom = actor({
  state: { messages: [] as string[] },
  actions: {
    getRoomInfo: (c) => ({ org: c.key[0], room: c.key[1] }),
  },
});

export const registry = setup({
  use: { chatRoom },
});

registry.start();
