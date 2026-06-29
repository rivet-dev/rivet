import { actor } from "rivetkit";

const myCustomRunHandler = (_options: Record<string, unknown>) => ({
  name: "My Custom Handler",
  icon: "bolt",
  run: async () => {},
});

const myActor = actor({
  run: myCustomRunHandler({ /* options */ }),
  // Picks up "My Custom Handler" name and "bolt" icon in registry metadata
});
