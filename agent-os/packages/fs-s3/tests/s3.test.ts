import { afterAll, beforeAll, describe } from "vitest";
import { defineFsDriverTests } from "@rivet-dev/agent-os/test/file-system";
import type { MinioContainerHandle } from "@rivet-dev/agent-os/test/docker";
import { startMinioContainer } from "@rivet-dev/agent-os/test/docker";
import { createS3Backend } from "../src/index.js";

let minio: MinioContainerHandle;

beforeAll(async () => {
	minio = await startMinioContainer({ healthTimeout: 60_000 });
}, 90_000);

afterAll(async () => {
	if (minio) await minio.stop();
});

defineFsDriverTests({
	name: "S3Backend (MinIO)",
	createFs: () => {
		// Use a unique prefix per test to avoid cross-test interference.
		const prefix = `test-${Date.now()}-${Math.random().toString(36).slice(2, 8)}/`;
		return createS3Backend({
			bucket: minio.bucket,
			prefix,
			region: "us-east-1",
			endpoint: minio.endpoint,
			credentials: {
				accessKeyId: minio.accessKeyId,
				secretAccessKey: minio.secretAccessKey,
			},
		});
	},
	capabilities: {
		symlinks: false,
		hardLinks: false,
		permissions: false,
		utimes: false,
		truncate: true,
		pread: true,
		mkdir: false,
		removeDir: false,
	},
});
