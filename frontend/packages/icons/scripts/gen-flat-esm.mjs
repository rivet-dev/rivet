#!/usr/bin/env node
// Generates dist/index.flat.js (flat ESM for tree-shaking) and
// dist/index.all.js (all-icons registry for dynamic lookup) from dist/index.js.
//
// Run after vendor-icons.js to update the flat ESM format.

import { writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const iconsDir = path.resolve(__dirname, "..");
const iconsPath = path.join(iconsDir, "dist", "index.js");

console.log("Loading icons module from dist/index.js...");
const iconsModule = await import(pathToFileURL(iconsPath).href).catch((err) => {
  console.error("Failed to import dist/index.js:", err.message);
  process.exit(1);
});

const entries = Object.entries(iconsModule).filter(
  ([k]) => k !== "default" && k !== "Icon" && k !== "FontAwesomeIcon"
);
console.log(`Loaded ${entries.length} icons`);

// --- Generate dist/index.flat.js ---
const flatLines = [
  `// @ts-nocheck`,
  `// Auto-generated flat ESM for optimal tree-shaking.`,
  `// Each icon is a direct export const so Rollup can tree-shake unused icons.`,
  `// Regenerate by running: pnpm gen:flat`,
  ``,
  `import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";`,
  `import { createElement } from "react";`,
  ``,
  `export function Icon(props) { return createElement(FontAwesomeIcon, props); }`,
  `export { FontAwesomeIcon };`,
  ``,
];
let iconCount = 0;
for (const [name, value] of entries) {
  if (typeof value !== "object" || value === null) continue;
  try {
    flatLines.push(`export const ${name} = ${JSON.stringify(value)};`);
    iconCount++;
  } catch (_) {
    // skip non-serializable values
  }
}
const flatContent = flatLines.join("\n") + "\n";
writeFileSync(path.join(iconsDir, "dist", "index.flat.js"), flatContent);
console.log(
  `Written dist/index.flat.js (${(flatContent.length / 1024).toFixed(0)} kB, ${iconCount} icons)`
);

// --- Generate dist/index.all.js ---
const allLines = [
  `// @ts-nocheck`,
  `// Auto-generated all-icons registry for runtime dynamic icon lookup.`,
  `// Import via: import("@rivet-gg/icons/all").then(m => m.default)`,
  `// For specific icons, use named imports from "@rivet-gg/icons" instead.`,
  `// Regenerate by running: pnpm gen:flat`,
  ``,
  `const icons = {`,
];
for (const [name, value] of entries) {
  if (typeof value !== "object" || value === null) continue;
  try {
    allLines.push(`  "${name}": ${JSON.stringify(value)},`);
  } catch (_) {
    // skip
  }
}
allLines.push(`};`, ``, `export default icons;`, ``);
const allContent = allLines.join("\n");
writeFileSync(path.join(iconsDir, "dist", "index.all.js"), allContent);
console.log(
  `Written dist/index.all.js (${(allContent.length / 1024).toFixed(0)} kB)`
);
