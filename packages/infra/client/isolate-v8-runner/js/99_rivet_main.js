import { core, primordials } from "ext:core/mod.js";
import { rivetNs } from "ext:rivet_runtime/90_rivet_ns.js";
const { ObjectDefineProperty } = primordials;

ObjectDefineProperty(globalThis, "Rivet", core.propReadOnly(rivetNs));
