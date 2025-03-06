import { createAndStartServer } from "../shared/server.js";

// Create and start server with default configuration
createAndStartServer().catch(err => {
  console.error("Failed to start server:", err);
  process.exit(1);
});
