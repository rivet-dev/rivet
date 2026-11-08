import { actor, setup } from "rivetkit";
import { pino } from "pino";

const counter = actor({
  vars: { count: 0 },
  actions: { increment: (c) => c.vars.count++ }
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
