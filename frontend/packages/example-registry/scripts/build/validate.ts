interface PackageJson {
	name: string;
	template?: {
		technologies: string[];
		tags: string[];
	};
	dependencies?: Record<string, string>;
	devDependencies?: Record<string, string>;
}

export function validateReadmeFormat(
	readmeContent: string,
	exampleName: string,
): { displayName: string; description: string } {
	const lines = readmeContent.split("\n");
	let displayName = "";
	let description = "";
	let foundTitle = false;

	for (const line of lines) {
		// Extract display name from title (e.g., "# Counter Example" -> "Counter Example")
		if (line.startsWith("# ")) {
			displayName = line.slice(2).trim();
			foundTitle = true;
			continue;
		}
		// Get first non-empty line after title as description
		if (foundTitle && line.trim() !== "") {
			description = line.trim();
			break;
		}
	}

	// Validate README format
	if (!displayName) {
		throw new Error(
			`README format validation failed for ${exampleName}: Missing title (# Heading)`,
		);
	}

	if (!description) {
		throw new Error(
			`README format validation failed for ${exampleName}: Missing description (first paragraph after title)`,
		);
	}

	return { displayName, description };
}

export function validateRivetKitVersions(
	packageJson: PackageJson,
	exampleName: string,
): void {
	const allDeps = {
		...packageJson.dependencies,
		...packageJson.devDependencies,
	};

	for (const [pkgName, version] of Object.entries(allDeps)) {
		// Check if it's a rivetkit or @rivetkit/* package
		if (pkgName === "rivetkit" || pkgName.startsWith("@rivetkit/")) {
			// Allow workspace:* for monorepo development
			if (version === "workspace:*") {
				continue;
			}

			// Otherwise, must be exactly "latest"
			if (version !== "latest") {
				throw new Error(
					`Package version validation failed for ${exampleName}: Package "${pkgName}" version must be "latest" (found "${version}")`,
				);
			}
		}
	}
}
