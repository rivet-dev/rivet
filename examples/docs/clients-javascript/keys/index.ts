import { actor, setup } from "rivetkit";

export const chatRoom = actor({
  actions: {
    getRoomInfo: (c) => ({ org: c.key[0], room: c.key[1] }),
  },
});

export const registry = setup({
  use: { chatRoom },
});

registry.start();
