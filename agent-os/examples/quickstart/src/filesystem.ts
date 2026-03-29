// Filesystem operations: mkdir, writeFile, readdir, stat, move, delete.

import { AgentOs } from "@rivet-dev/agent-os-core";

const vm = await AgentOs.create();

// Create a directory structure
await vm.mkdir("/project");
await vm.mkdir("/project/src");
await vm.writeFile("/project/src/index.ts", 'console.log("hello");');
await vm.writeFile("/project/README.md", "# My Project");

// List directory contents (filter out . and ..)
const entries = await vm.readdir("/project");
console.log(
	"project/:",
	entries.filter((e) => e !== "." && e !== ".."),
);

// Stat a file
const info = await vm.stat("/project/src/index.ts");
console.log("index.ts size:", info.size, "isDirectory:", info.isDirectory);

// Check existence
console.log("/project exists:", await vm.exists("/project"));
console.log("/missing exists:", await vm.exists("/missing"));

// Move a file
await vm.move("/project/README.md", "/project/docs.md");
console.log("docs.md exists:", await vm.exists("/project/docs.md"));
console.log("README.md exists:", await vm.exists("/project/README.md"));

// Delete a file, then delete directory recursively
await vm.delete("/project/docs.md");
await vm.delete("/project", { recursive: true });
console.log("project exists after delete:", await vm.exists("/project"));

await vm.dispose();
