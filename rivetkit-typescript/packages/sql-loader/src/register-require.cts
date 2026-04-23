import { readFileSync } from "node:fs";

require.extensions[".sql"] = (module, filename) => {
	module.exports = readFileSync(filename, "utf-8");
};
