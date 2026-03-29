import { defineFsDriverTests } from "@rivet-dev/agent-os/test/file-system";
import { createSqliteBackend } from "../src/index.js";

defineFsDriverTests({
	name: "SqliteBackend",
	createFs: () => {
		return createSqliteBackend({ dbPath: ":memory:" });
	},
	capabilities: {
		symlinks: true,
		hardLinks: true,
		permissions: true,
		utimes: true,
		truncate: true,
		pread: true,
		mkdir: true,
		removeDir: true,
	},
});
