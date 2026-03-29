import { afterAll, beforeAll } from "vitest";
import { defineBlockStoreTests } from "@secure-exec/core/test/block-store-conformance";
import { defineVfsConformanceTests } from "@secure-exec/core/test/vfs-conformance";
import { createChunkedVfs, SqliteMetadataStore } from "@secure-exec/core";
import type { MinioContainerHandle } from "@rivet-dev/agent-os-core/test/docker";
import { startMinioContainer } from "@rivet-dev/agent-os-core/test/docker";
import { S3BlockStore } from "../src/index.js";

let minio: MinioContainerHandle;

beforeAll(async () => {
	minio = await startMinioContainer({ healthTimeout: 60_000 });
}, 90_000);

afterAll(async () => {
	if (minio) await minio.stop();
});

function createStore(): S3BlockStore {
	const prefix = `test-${Date.now()}-${Math.random().toString(36).slice(2, 8)}/`;
	return new S3BlockStore({
		bucket: minio.bucket,
		prefix,
		region: "us-east-1",
		endpoint: minio.endpoint,
		credentials: {
			accessKeyId: minio.accessKeyId,
			secretAccessKey: minio.secretAccessKey,
		},
	});
}

// Block store conformance tests.
defineBlockStoreTests({
	name: "S3BlockStore (MinIO)",
	createStore,
	capabilities: {
		copy: true,
	},
});

// VFS conformance tests with ChunkedVFS(SqliteMetadataStore + S3BlockStore).
const INLINE_THRESHOLD = 256;
const CHUNK_SIZE = 1024;

defineVfsConformanceTests({
	name: "ChunkedVFS (SqliteMetadata + S3BlockStore)",
	createFs: () =>
		createChunkedVfs({
			metadata: new SqliteMetadataStore({ dbPath: ":memory:" }),
			blocks: createStore(),
			inlineThreshold: INLINE_THRESHOLD,
			chunkSize: CHUNK_SIZE,
		}),
	capabilities: {
		symlinks: true,
		hardLinks: true,
		permissions: true,
		utimes: true,
		truncate: true,
		pread: true,
		pwrite: true,
		mkdir: true,
		removeDir: true,
		fsync: false,
		copy: true,
		readDirStat: true,
	},
	inlineThreshold: INLINE_THRESHOLD,
	chunkSize: CHUNK_SIZE,
});
