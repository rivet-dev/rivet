import { register } from "node:module";
import { pathToFileURL } from "node:url";

const isCJS = typeof module !== "undefined" && typeof exports !== "undefined";

register(
	isCJS ? "./hook.cjs" : "./hook.js",
	pathToFileURL(import.meta.filename),
);
