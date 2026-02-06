export const migrations = {
	journal: {
		entries: [
			{
				idx: 0,
				when: 1700000000000,
				tag: "0000_init",
				breakpoints: false,
			},
		],
	},
	migrations: {
		m0000: `
			CREATE TABLE IF NOT EXISTS test_data (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				value TEXT NOT NULL,
				payload TEXT NOT NULL DEFAULT '',
				created_at INTEGER NOT NULL
			);
		`,
	},
};
