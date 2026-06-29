import { actor, setup } from "rivetkit";
import { pino } from "pino";

const counter = actor({
  state: { count: 0 },
  actions: { increment: (c) => c.state.count++ }
});

const customLogger = pino({
  level: "info",
  transport: {
    target: "pino-pretty"
  }
});

const registry = setup({
  use: { counter },
  logging: {
    baseLogger: customLogger,  // Use your custom Pino logger
  }
});
