import { actor, setup } from "rivetkit";

export const user = actor({
  state: { name: "" },
  actions: {},
});

export const document = actor({
  state: { content: "" },
  actions: {},
});

export const registry = setup({ use: { user, document } });

registry.start();
