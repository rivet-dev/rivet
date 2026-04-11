import journal from './meta/_journal.json' with { type: 'json' };

const m0000 = `CREATE TABLE \`todos\` (
\t\`id\` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
\t\`title\` text NOT NULL,
\t\`completed\` integer DEFAULT 0,
\t\`created_at\` integer NOT NULL
);`;

export default {
  journal,
  migrations: {
    m0000
  }
}
