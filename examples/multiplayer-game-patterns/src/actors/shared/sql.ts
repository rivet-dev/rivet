export function sqlString(value: string): string {
	return `'${value.replaceAll("'", "''")}'`;
}

export function sqlInt(value: number): string {
	if (!Number.isFinite(value)) {
		throw new Error("invalid sql integer");
	}
	return `${Math.trunc(value)}`;
}
