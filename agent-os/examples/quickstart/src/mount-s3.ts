// Mount an S3 bucket into the VM filesystem.
//
// This demonstrates the concept of mounting external storage into the VM.
// The actual S3 filesystem backend is not yet implemented — this is a
// placeholder showing the intended API.
//
// When implemented, agents will be able to read/write S3 objects as if
// they were local files inside the VM.

import { AgentOs } from "@rivet-dev/agent-os-core";

const vm = await AgentOs.create();

// TODO: S3 mount is not yet implemented. The intended API:
//
// await vm.mount("/mnt/data", {
//   type: "s3",
//   bucket: "my-bucket",
//   prefix: "datasets/",
//   region: "us-east-1",
//   credentials: {
//     accessKeyId: process.env.AWS_ACCESS_KEY_ID!,
//     secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY!,
//   },
// });
//
// // Agents see S3 objects as regular files
// const files = await vm.readdir("/mnt/data");
// const content = await vm.readFile("/mnt/data/input.csv");
//
// // Writes go back to S3
// await vm.writeFile("/mnt/data/output.csv", "col1,col2\na,b\n");

// For now, demonstrate the filesystem pattern that S3 mount will follow:
await vm.mkdir("/mnt/data");
await vm.writeFile("/mnt/data/input.csv", "name,score\nalice,95\nbob,87\n");

const content = await vm.readFile("/mnt/data/input.csv");
console.log(`File contents:\n${new TextDecoder().decode(content)}`);

const entries = await vm.readdir("/mnt/data");
console.log(
	"Files:",
	entries.filter((e) => e !== "." && e !== ".."),
);

await vm.dispose();
