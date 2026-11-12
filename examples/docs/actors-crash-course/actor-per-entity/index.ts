import { actor, setup } from "rivetkit";

export const user = actor({
  vars: { name: "" },
  actions: {},
});

export const document = actor({
  vars: { content: "" },
  actions: {},
});

export const registry = setup({ use: { user, document } });

registry.start();
