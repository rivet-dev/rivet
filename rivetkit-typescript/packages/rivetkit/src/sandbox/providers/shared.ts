export async function importOptionalDependency<TModule>(
	packageName: string,
	providerName: string,
): Promise<TModule> {
	try {
		return (await import(packageName)) as TModule;
	} catch (error) {
		if (
			error instanceof Error &&
			("code" in error || "message" in error) &&
			((error as { code?: string }).code === "ERR_MODULE_NOT_FOUND" ||
				error.message.includes(`'${packageName}'`) ||
				error.message.includes(`"${packageName}"`))
		) {
			throw new Error(
				`sandbox provider "${providerName}" requires the optional dependency "${packageName}". Install it with \`pnpm add ${packageName}\`.`,
			);
		}

		throw error;
	}
}
