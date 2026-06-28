import { actor } from "rivetkit";

const myActor = actor({
  state: { count: 0 },

  options: {
    // Timeout for createVars function (default: 5000ms)
    createVarsTimeout: 5000,

    // Timeout for createConnState function (default: 5000ms)
    createConnStateTimeout: 5000,

    // Timeout for onConnect hook (default: 5000ms)
    onConnectTimeout: 5000,

    // Total graceful shutdown budget for both sleep and destroy. Default: 15000ms.
    sleepGracePeriod: 15_000,

    // Interval for saving state (default: 1000ms)
    stateSaveInterval: 1_000,

    // Timeout for action execution (default: 60000ms)
    actionTimeout: 60_000,

    // Timeout for connection liveness check (default: 2500ms)
    connectionLivenessTimeout: 2500,

    // Interval for connection liveness check (default: 5000ms)
    connectionLivenessInterval: 5000,

    // Time before actor sleeps due to inactivity (default: 30000ms)
    sleepTimeout: 30_000,

    // Whether WebSockets can hibernate for onWebSocket (default: false)
    // Can be a boolean or a function that takes a Request and returns a boolean
    canHibernateWebSocket: false,
  },

  actions: { /* ... */ }
});
