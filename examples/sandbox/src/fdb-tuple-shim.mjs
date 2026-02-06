import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const fdbTuple = require("fdb-tuple");
export const { concat2, unboundVersionstamp, rawRange, pack, packUnboundVersionstamp, name, unpack, range, bakeVersionstamp } = fdbTuple;
export default fdbTuple;
