import { setup } from "rivetkit";

const rivet = setup({
  use: { /* ... */ },
  maxIncomingMessageSize: 1_048_576,
  maxOutgoingMessageSize: 10_485_760,
});
