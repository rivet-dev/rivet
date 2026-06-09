import { bootstrapBridgeChild } from "./child-main";

// Production worker entry for bridged actors; bundled and exported as
// rivetkit/bridge-child. Definitions resolve via runtime import.
void bootstrapBridgeChild();
