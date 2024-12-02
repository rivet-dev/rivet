import { core, primordials } from "ext:core/mod.js";
import { RIVET_NAMESPACE } from "ext:rivet_runtime/90_rivet_ns.js";
const { ObjectDefineProperty } = primordials;
ObjectDefineProperty(globalThis, "Rivet", core.propReadOnly(RIVET_NAMESPACE));
