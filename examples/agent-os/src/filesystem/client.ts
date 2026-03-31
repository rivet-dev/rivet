// Filesystem operations: write, read, mkdir, readdir, stat, move, delete.
//
// The VM mounts a filesystem at /home/user by default. Custom mounts
// (S3, host directories) can be configured in the server.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

// Create a directory structure
await agent.mkdir("/home/user/project");
await agent.mkdir("/home/user/project/src");
await agent.writeFile(
	"/home/user/project/src/index.ts",
	'console.log("hello");',
);
await agent.writeFile("/home/user/project/README.md", "# My Project");

// List directory contents (filter out . and ..)
const entries = (await agent.readdir("/home/user/project")) as string[];
console.log(
	"project/:",
	entries.filter((e) => e !== "." && e !== ".."),
);

// Stat a file
const info = (await agent.stat("/home/user/project/src/index.ts")) as {
	size: number;
	isDirectory: boolean;
};
console.log("index.ts size:", info.size, "isDirectory:", info.isDirectory);

// Recursive directory listing
const tree = (await agent.readdirRecursive("/home/user/project", {
	maxDepth: 3,
})) as Array<{ path: string; isDirectory: boolean }>;
console.log("Recursive listing:", tree);

// Check existence
console.log("/home/user/project exists:", await agent.exists("/home/user/project"));
console.log("/missing exists:", await agent.exists("/missing"));

// Move a file
await agent.move("/home/user/project/README.md", "/home/user/project/docs.md");
console.log(
	"docs.md exists:",
	await agent.exists("/home/user/project/docs.md"),
);

// Delete a file, then delete directory recursively
await agent.deleteFile("/home/user/project/docs.md");
await agent.deleteFile("/home/user/project", { recursive: true });
console.log(
	"project exists after delete:",
	await agent.exists("/home/user/project"),
);
