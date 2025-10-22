import { TemplateContext } from "../../context";

export function generateDatacenterPostgres(
	context: TemplateContext,
	dcId: string,
) {
	// Generate a basic PostgreSQL initialization script
	const initScript = `#!/bin/bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    CREATE DATABASE rivet_engine;
EOSQL
`;

	context.writeDatacenterServiceFile(
		"postgres",
		dcId,
		"init-db.sh",
		initScript,
	);
	context.makeDatabaseFileExec("postgres", dcId, "init-db.sh");
}