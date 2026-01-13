interface PackageJson {
	name: string;
	license?: string;
	template?: {
		technologies: string[];
		tags: string[];
		noFrontend?: boolean;
		frontendPort?: number;
	};
	scripts?: Record<string, string>;
	dependencies?: Record<string, string>;
	devDependencies?: Record<string, string>;
}

const MAX_TITLE_LENGTH = 60;
const MAX_DESCRIPTION_LENGTH = 200;

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

	// Validate title length
	if (displayName.length > MAX_TITLE_LENGTH) {
		throw new Error(
			`README format validation failed for ${exampleName}: Title too long (${displayName.length} > ${MAX_TITLE_LENGTH}). Title: "${displayName}"`,
		);
	}

	// Check for "rivet" or "rivetkit" in title (case-insensitive)
	if (/rivet/i.test(displayName)) {
		throw new Error(
			`README format validation failed for ${exampleName}: Title should not contain "rivet" or "rivetkit". Title: "${displayName}"`,
		);
	}

	// Check for "Learn More" section
	if (readmeContent.includes("[Learn More")) {
		throw new Error(
			`README format validation failed for ${exampleName}: README should not contain "Learn More" section`,
		);
	}

	// Check for Discord/Documentation/Issues links
	if (
		readmeContent.includes("[Discord]") &&
		readmeContent.includes("[Documentation]") &&
		readmeContent.includes("[Issues]")
	) {
		throw new Error(
			`README format validation failed for ${exampleName}: README should not contain Discord/Documentation/Issues links section`,
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
			if (version !== "*") {
				throw new Error(
					`Package version validation failed for ${exampleName}: Package "${pkgName}" version must be "*" (found "${version}")`,
				);
			}
		}
	}
}

export function validatePackageJson(
	packageJson: PackageJson,
	exampleName: string,
): void {
	// Check for MIT license
	if (packageJson.license !== "MIT") {
		throw new Error(
			`Package.json validation failed for ${exampleName}: license must be "MIT" (found "${packageJson.license || 'none'}")`,
		);
	}

	// Check for required scripts
	if (!packageJson.scripts) {
		throw new Error(
			`Package.json validation failed for ${exampleName}: Missing scripts section`,
		);
	}

	if (!packageJson.scripts.dev) {
		throw new Error(
			`Package.json validation failed for ${exampleName}: Missing "dev" script`,
		);
	}

	if (!packageJson.scripts["check-types"]) {
		throw new Error(
			`Package.json validation failed for ${exampleName}: Missing "check-types" script`,
		);
	}
}

export async function validateTurboJson(
	exampleDir: string,
	exampleName: string,
): Promise<void> {
	const turboJsonPath = `${exampleDir}/turbo.json`;
	try {
		const fs = await import("node:fs/promises");
		await fs.access(turboJsonPath);
	} catch {
		throw new Error(
			`Validation failed for ${exampleName}: Missing turbo.json file`,
		);
	}
}

export function validateFrontendConfig(
	packageJson: PackageJson,
	exampleName: string,
): void {
	const template = packageJson.template;
	if (!template) {
		return; // Will be caught by other validation
	}

	const { noFrontend, frontendPort } = template;

	// Must have either noFrontend: true OR a frontendPort specified
	if (noFrontend && frontendPort !== undefined) {
		throw new Error(
			`Validation failed for ${exampleName}: Cannot specify both "noFrontend: true" and "frontendPort" in template config`,
		);
	}

	if (!noFrontend && frontendPort === undefined) {
		throw new Error(
			`Validation failed for ${exampleName}: Must specify either "noFrontend: true" or "frontendPort" in template config`,
		);
	}

	// Validate frontendPort is a valid port number
	if (frontendPort !== undefined) {
		if (!Number.isInteger(frontendPort) || frontendPort < 1 || frontendPort > 65535) {
			throw new Error(
				`Validation failed for ${exampleName}: "frontendPort" must be a valid port number (1-65535), got ${frontendPort}`,
			);
		}
	}
}
