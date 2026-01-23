"use node";

import { action } from "./_generated/server";
import { createRivetAction } from "@rivetkit/convex";
import { registry } from "./rivet/actors";

export const handle = action(createRivetAction(registry));
