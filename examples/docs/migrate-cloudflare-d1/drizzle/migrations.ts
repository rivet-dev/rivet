// Stand-in for the migrations.js file you write next to drizzle-kit's output.
// See the SQLite + Drizzle docs for the real project layout.
export default {
	journal: {
		version: "7",
		dialect: "sqlite",
		entries: [
			{
				idx: 0,
				version: "7",
				when: 1735689600000,
				tag: "0000_init",
				breakpoints: true,
			},
		],
	},
	migrations: {
		m0000: `CREATE TABLE \`users\` (
	\`id\` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	\`email\` text NOT NULL,
	\`name\` text NOT NULL
);
CREATE UNIQUE INDEX \`users_email_unique\` ON \`users\` (\`email\`);`,
	},
};
