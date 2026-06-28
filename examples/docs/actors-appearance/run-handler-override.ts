import { actor } from "rivetkit";

const myCustomRunHandler = (_options: Record<string, unknown>) => ({
  name: "My Custom Handler",
  icon: "bolt",
  run: async () => {},
});

const myActor = actor({
  options: {
    name: "Custom Name",  // Overrides "My Custom Handler"
    icon: "rocket",       // Overrides "bolt"
  },
  run: myCustomRunHandler({ /* options */ }),
});
