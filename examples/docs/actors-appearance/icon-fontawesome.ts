import { actor } from "rivetkit";

const gameServer = actor({
  options: {
    name: "Game Server",
    icon: "gamepad",
  },
  // ...
});

const analyticsWorker = actor({
  options: {
    name: "Analytics",
    icon: "chart-line",
  },
  // ...
});
