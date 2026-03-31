import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import common from "@rivet-dev/agent-os-common";

// The default agentOS actor mounts an in-memory filesystem at /home/user.
// You can add custom mounts for S3, host directories, or other backends:
//
//   import { S3BlockStore } from "@rivet-dev/agent-os-s3";
//   const vm = agentOs({
//     options: {
//       software: [common],
//       mounts: [{
//         path: "/data",
//         driver: createChunkedVfs(sqliteMetadata, new S3BlockStore({ bucket: "my-bucket" })),
//       }],
//     },
//   });
//
// For this example, we use the default in-memory mount.

const vm = agentOs({ options: { software: [common] } });

export const registry = setup({ use: { vm } });
registry.start();
