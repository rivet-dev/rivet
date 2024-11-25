/**
 * This file re-exports Rivet from the global scope as a module dependency.
 *
 * This allows developers to import Rivet with `import Rivet from "@rivet-gg/actors-core"`.
 *
 * An alternative approach is to use `declare module Rivet` then have developers
 * use `import "path/to/types.d.ts"`. However, this is rather annoying to do in
 * every file that uses Rivet. Additionally, it's not something most JS
 * developers are used to.
 */

import type { RIVET_NAMESPACE } from "./types/90_rivet_ns.d.ts";

const Rivet = (globalThis as any).Rivet as typeof RIVET_NAMESPACE;
if (!Rivet) {
  throw new Error(
    "Rivet is not defined in the global scope. This likely means this script is not being ran as a Rivet Actor.",
  );
}

export default Rivet;

