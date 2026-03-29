/**
 * Google Drive block store conformance tests.
 *
 * These tests require real Google Drive API credentials and a folder ID.
 * Set the following environment variables to run:
 *   GOOGLE_DRIVE_CLIENT_EMAIL - Service account email
 *   GOOGLE_DRIVE_PRIVATE_KEY  - Service account private key (PEM)
 *   GOOGLE_DRIVE_FOLDER_ID    - Folder ID where test files are stored
 *
 * When credentials are not set, tests are skipped with a descriptive message.
 */

import { describe, it } from "vitest";
import { defineBlockStoreTests } from "@secure-exec/core/test/block-store-conformance";
import { defineVfsConformanceTests } from "@secure-exec/core/test/vfs-conformance";
import { createChunkedVfs, SqliteMetadataStore } from "@secure-exec/core";
import { GoogleDriveBlockStore } from "../src/index.js";

const clientEmail = process.env.GOOGLE_DRIVE_CLIENT_EMAIL;
const privateKey = process.env.GOOGLE_DRIVE_PRIVATE_KEY;
const folderId = process.env.GOOGLE_DRIVE_FOLDER_ID;

const hasCredentials = !!(clientEmail && privateKey && folderId);

if (hasCredentials) {
	function createStore(): GoogleDriveBlockStore {
		const prefix = `test-${Date.now()}-${Math.random().toString(36).slice(2, 8)}/`;
		return new GoogleDriveBlockStore({
			credentials: {
				clientEmail: clientEmail!,
				privateKey: privateKey!,
			},
			folderId: folderId!,
			keyPrefix: prefix,
		});
	}

	// Block store conformance tests.
	defineBlockStoreTests({
		name: "GoogleDriveBlockStore",
		createStore,
		capabilities: {
			copy: true,
		},
	});

	// VFS conformance tests with ChunkedVFS(SqliteMetadataStore + GoogleDriveBlockStore).
	const INLINE_THRESHOLD = 256;
	const CHUNK_SIZE = 1024;

	defineVfsConformanceTests({
		name: "ChunkedVFS (SqliteMetadata + GoogleDriveBlockStore)",
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
} else {
	describe("GoogleDriveBlockStore", () => {
		it.skip("skipped: set GOOGLE_DRIVE_CLIENT_EMAIL, GOOGLE_DRIVE_PRIVATE_KEY, and GOOGLE_DRIVE_FOLDER_ID to run", () => {});
	});

	describe("ChunkedVFS (SqliteMetadata + GoogleDriveBlockStore)", () => {
		it.skip("skipped: set GOOGLE_DRIVE_CLIENT_EMAIL, GOOGLE_DRIVE_PRIVATE_KEY, and GOOGLE_DRIVE_FOLDER_ID to run", () => {});
	});
}
