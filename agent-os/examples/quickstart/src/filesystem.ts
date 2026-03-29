// Filesystem operations: write, read, mkdir, readdir, stat, move, delete.
//
// The VM creates an in-memory filesystem by default. Custom mounts
// (S3, host directories) can be configured at boot:
//
//   import { S3BlockStore } from "@rivet-dev/agent-os-s3";
//   const vm = await AgentOs.create({
//     mounts: [{
//       path: "/data",
//       driver: createChunkedVfs(sqliteMetadata, new S3BlockStore({ bucket: "my-bucket" })),
//     }],
//   });

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

// Recursive directory listing
const tree = await vm.readdirRecursive("/project", { maxDepth: 3 });
console.log("Recursive listing:", tree);

// Check existence
console.log("/project exists:", await vm.exists("/project"));
console.log("/missing exists:", await vm.exists("/missing"));

// Move a file
await vm.move("/project/README.md", "/project/docs.md");
console.log("docs.md exists:", await vm.exists("/project/docs.md"));

// Delete a file, then delete directory recursively
await vm.delete("/project/docs.md");
await vm.delete("/project", { recursive: true });
console.log("project exists after delete:", await vm.exists("/project"));

await vm.dispose();
