import { actor } from "rivetkit";

// Chat/messaging actors
const chatRoom = actor({
  options: { name: "Chat Room", icon: "comments" },
  // ...
});

// Game-related actors
const matchmaker = actor({
  options: { name: "Matchmaker", icon: "users" },
  // ...
});

const gameSession = actor({
  options: { name: "Game Session", icon: "gamepad" },
  // ...
});

// Data processing actors
const dataProcessor = actor({
  options: { name: "Data Processor", icon: "microchip" },
  // ...
});

// Using emojis for quick identification
const alertService = actor({
  options: { name: "Alerts", icon: "🚨" },
  // ...
});
