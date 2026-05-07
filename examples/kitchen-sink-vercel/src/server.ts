import { registry } from "./index.ts";

const fetch = registry.fetchHandler({ path: "/api/rivet" });

export default { fetch };
