export const extractPostgressCredentials = (connectionString: string) => {
	const regex = /postgres:\/\/([^:]+):([^@]+)@([^/]+)\/([^?]+)/;
	const match = connectionString.match(regex);
	if (!match) {
		return null;
	}
	const [, user, password, host, database] = match;
	const port = "5432"; // Assuming default port is 5432 for PostgreSQL
	const type = "postgres";

	return { type, port, host, user, password, database };
};
