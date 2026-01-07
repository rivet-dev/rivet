#!/usr/bin/env node
// @ts-nocheck
//
// RIVET ICONS MANIFEST GENERATOR
// This script is for Rivet maintainers only.
// It scans Font Awesome packages and generates a manifest of available icons.
//
// LEGAL NOTICE: The generated manifest is for use in Rivet products only.
// Using this in any other product or project is strictly prohibited.
//

const fs = require("node:fs");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");
const { getPackageInfo, importModule, resolveModule } = require("local-pkg");
const dedentModule = require("dedent");
const dedent = dedentModule.default || dedentModule;

// ============================================================================
// CONFIGURATION
// ============================================================================

const PATHS = {
	root: join(__dirname, ".."),
	get src() {
		return join(this.root, "src");
	},
	get srcNodeModules() {
		return join(this.src, "node_modules");
	},
	get rootNodeModules() {
		return join(this.root, "..", "..", "..", "node_modules");
	},
	get manifest() {
		return join(this.root, "manifest.json");
	},
};

const SEARCH_PATHS = [PATHS.srcNodeModules, PATHS.rootNodeModules];

const FA_PACKAGES_CONFIG = {
	"@awesome.me/kit-63db24046b": "1.0.27",
	"@fortawesome/pro-regular-svg-icons": "6.6.0",
	"@fortawesome/pro-solid-svg-icons": "6.6.0",
	"@fortawesome/free-solid-svg-icons": "6.6.0",
	"@fortawesome/free-brands-svg-icons": "6.6.0",
};

const FA_PACKAGES = [
	"@fortawesome/free-solid-svg-icons",
	"@fortawesome/free-brands-svg-icons",
	"@fortawesome/pro-solid-svg-icons",
];

const CUSTOM_KITS = ["@awesome.me/kit-63db24046b/icons/kit/custom"];

// Track globally registered icons to avoid duplicates
const registeredIcons = new Set();

// ============================================================================
// UTILITIES
// ============================================================================

/**
 * @param {string} emoji
 * @param {string} message
 */
function log(emoji, message) {
	console.log(`${emoji} ${message}`);
}

/**
 * @param {string} message
 */
function error(message) {
	console.error(`❌ ${message}`);
}

/**
 * @param {string} message
 */
function exitWithError(message) {
	error(message);
	process.exit(1);
}

/**
 * Converts kebab-case to faCamelCase
 * @param {string} str - Icon name in kebab-case
 * @returns {string} Icon name in faCamelCase
 * @example
 * faCamelCase("arrow-left") // => "faArrowLeft"
 */
function faCamelCase(str) {
	const camelCase = str.replace(/-./g, (g) => g[1].toUpperCase());
	const [firstLetter, ...restLetters] = camelCase;
	return `fa${firstLetter.toUpperCase()}${restLetters.join("")}`;
}

// ============================================================================
// SETUP PHASE
// ============================================================================

function checkEnvironment() {
	log("🔍", "Checking environment...");
	if (!process.env.FONTAWESOME_PACKAGE_TOKEN) {
		exitWithError(
			"FONTAWESOME_PACKAGE_TOKEN environment variable is required.\n" +
				"This script should only be run by maintainers with a Font Awesome Pro license.",
		);
	}
	log("🔑", "Font Awesome token found");
}

function setupSourceDirectory() {
	log("📁", "Setting up source directory...");
	if (!fs.existsSync(PATHS.src)) {
		fs.mkdirSync(PATHS.src, { recursive: true });
		log("✨", `Created directory: ${PATHS.src}`);
	}
}

function configureFontAwesomeRegistry() {
	log("📝", "Configuring Font Awesome registry...");

	// Create .npmrc for Font Awesome Pro authentication
	const npmrcContent = dedent`
		@fortawesome:registry=https://npm.fontawesome.com/
		@awesome.me:registry=https://npm.fontawesome.com/
		//npm.fontawesome.com/:_authToken=\${FONTAWESOME_PACKAGE_TOKEN}
		//npm.fontawesome.com/:always-auth=true
	`;
	fs.writeFileSync(join(PATHS.src, ".npmrc"), npmrcContent);

	// Create temporary package.json
	const packageJson = {
		name: "@rivet-gg/internal-icons",
		private: true,
		sideEffects: false,
		dependencies: FA_PACKAGES_CONFIG,
	};
	fs.writeFileSync(
		join(PATHS.src, "package.json"),
		JSON.stringify(packageJson, null, 2),
	);

	log("✅", "Registry configured");
}

function installFontAwesomePackages() {
	log("📦", "Installing Font Awesome Pro packages...");
	log("⏳", "This may take a minute...");

	const result = spawnSync("npm", ["install", "--no-package-lock", "--silent"], {
		stdio: "inherit",
		cwd: PATHS.src,
		env: { ...process.env, CI: "0" },
	});

	if (result.status !== 0) {
		exitWithError("Failed to install Font Awesome packages");
	}

	log("✅", "Packages installed");
}

// ============================================================================
// FONT AWESOME ICON REGISTRATION
// ============================================================================

/**
 * Registers all icons from a Font Awesome package
 * @param {string} packageName - Name of the FA package to register
 * @returns {Promise<Record<string, {icons: Array, prefix: string}>>}
 */
async function registerFontAwesomePackage(packageName) {
	log("📦", `Processing ${packageName}...`);

	// Find the package
	const packageInfo = await getPackageInfo(packageName, {
		paths: SEARCH_PATHS,
	});

	if (!packageInfo) {
		throw new Error(`Could not find package: ${packageName}`);
	}

	const { rootPath } = packageInfo;

	// Resolve the module
	const modulePath = resolveModule(packageName, { paths: [rootPath] });
	if (!modulePath) {
		throw new Error(`Could not resolve module: ${packageName}`);
	}

	// Read icon files from package directory
	const files = await fs.promises.readdir(rootPath);
	const iconFiles = files.filter(
		(file) => file.startsWith("fa") && file.endsWith(".js"),
	);

	// Import the module to get icon metadata
	const iconsModule = await importModule(modulePath);
	const foundIcons = [];
	let skippedCount = 0;

	// First pass: collect all icons and group by iconBaseName
	const iconsByBaseName = new Map();

	for (const iconFile of iconFiles) {
		const iconName = iconFile.replace(".js", "");
		const iconDefinition = iconsModule[iconName];

		if (!iconDefinition) {
			continue;
		}

		const iconBaseName = iconDefinition.iconName;

		// Skip if this iconBaseName is already registered globally
		if (registeredIcons.has(iconBaseName)) {
			skippedCount++;
			continue;
		}

		// Group icons by their base name
		if (!iconsByBaseName.has(iconBaseName)) {
			iconsByBaseName.set(iconBaseName, []);
		}
		iconsByBaseName.get(iconBaseName).push({ iconName, iconDefinition });
	}

	// Second pass: for each iconBaseName, prefer canonical icon
	for (const [iconBaseName, icons] of iconsByBaseName.entries()) {
		// Find the canonical icon (where filename matches iconBaseName in camelCase)
		const expectedCanonicalName = faCamelCase(iconBaseName);
		const canonicalIcon =
			icons.find((i) => i.iconName === expectedCanonicalName) || icons[0];

		const { iconName, iconDefinition } = canonicalIcon;

		// Extract aliases from the icon definition
		const faAliases = (iconDefinition.icon?.[2] || [])
			.filter((alias) => typeof alias === "string")
			.map(faCamelCase);

		// Build complete aliases list: canonical name + FA aliases
		const allAliases = [iconName, ...faAliases].filter(
			(alias, index, arr) => arr.indexOf(alias) === index,
		);

		// Register the icon
		registeredIcons.add(iconBaseName);
		foundIcons.push({ icon: iconName, aliases: allAliases });
	}

	log(
		"✅",
		`Found ${foundIcons.length} icons${skippedCount > 0 ? ` (skipped ${skippedCount} duplicates)` : ""}`,
	);

	return {
		[packageName]: {
			icons: foundIcons,
			prefix: iconsModule.prefix,
		},
	};
}

/**
 * Registers icons from a custom Font Awesome kit
 * @param {string} kitPath - Path to the custom kit
 * @returns {Record<string, {icons: Array}>}
 */
function registerCustomKit(kitPath) {
	log("📦", `Processing custom kit: ${kitPath}...`);

	// Resolve the custom kit module
	const modulePath = require.resolve(kitPath, { paths: SEARCH_PATHS });

	if (!modulePath) {
		throw new Error(`Could not resolve custom kit: ${kitPath}`);
	}

	// Load the custom icons
	const customIcons = require(modulePath);
	const foundIcons = [];
	let skippedCount = 0;

	// First pass: collect all icons and group by iconBaseName
	const iconsByBaseName = new Map();

	for (const [iconName, iconDefinition] of Object.entries(customIcons)) {
		if (!iconDefinition) {
			continue;
		}

		const iconBaseName = iconDefinition.iconName;

		// Skip if this iconBaseName is already registered globally
		if (registeredIcons.has(iconBaseName)) {
			skippedCount++;
			continue;
		}

		// Group icons by their base name
		if (!iconsByBaseName.has(iconBaseName)) {
			iconsByBaseName.set(iconBaseName, []);
		}
		iconsByBaseName.get(iconBaseName).push({ iconName, iconDefinition });
	}

	// Second pass: for each iconBaseName, prefer canonical icon
	for (const [iconBaseName, icons] of iconsByBaseName.entries()) {
		// Find the canonical icon (where filename matches iconBaseName in camelCase)
		const expectedCanonicalName = faCamelCase(iconBaseName);
		const canonicalIcon =
			icons.find((i) => i.iconName === expectedCanonicalName) || icons[0];

		const { iconName, iconDefinition } = canonicalIcon;

		// Extract aliases from the icon definition
		const faAliases = (iconDefinition.icon?.[2] || [])
			.filter((alias) => typeof alias === "string")
			.map(faCamelCase);

		// Build complete aliases list: canonical name + FA aliases
		const allAliases = [iconName, ...faAliases].filter(
			(alias, index, arr) => arr.indexOf(alias) === index,
		);

		// Register the icon
		registeredIcons.add(iconBaseName);
		foundIcons.push({ icon: iconName, aliases: allAliases });
	}

	log(
		"✅",
		`Found ${foundIcons.length} custom icons${skippedCount > 0 ? ` (skipped ${skippedCount} duplicates)` : ""}`,
	);

	return {
		[kitPath]: {
			icons: foundIcons,
		},
	};
}

// ============================================================================
// MAIN
// ============================================================================

async function main() {
	console.log("\n🔨 Rivet Icons Manifest Generator\n");

	// Setup phase
	checkEnvironment();
	setupSourceDirectory();
	configureFontAwesomeRegistry();
	installFontAwesomePackages();

	console.log();

	const manifest = {};
	let totalIcons = 0;

	// Register Font Awesome packages
	log("📚", "Registering Font Awesome packages...");
	for (const packageName of FA_PACKAGES) {
		try {
			const packageManifest = await registerFontAwesomePackage(packageName);
			Object.assign(manifest, packageManifest);
			totalIcons += packageManifest[packageName].icons.length;
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err);
			error(`Failed to register ${packageName}: ${message}`);
			throw err;
		}
	}

	console.log();

	// Register custom kits
	log("🎨", "Registering custom kits...");
	for (const kitPath of CUSTOM_KITS) {
		try {
			const kitManifest = registerCustomKit(kitPath);
			Object.assign(manifest, kitManifest);
			totalIcons += kitManifest[kitPath].icons.length;
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err);
			error(`Failed to register custom kit ${kitPath}: ${message}`);
			throw err;
		}
	}

	console.log();

	// Write manifest to file
	log("💾", "Writing manifest.json...");
	const manifestJson = JSON.stringify(manifest, null, 2);
	fs.writeFileSync(PATHS.manifest, manifestJson);

	const sizeKB = (Buffer.byteLength(manifestJson, "utf8") / 1024).toFixed(2);
	log("✅", `Manifest written (${sizeKB} KB)`);

	// Success summary
	console.log("\n🎉 Done!");
	console.log(`📊 Total icons registered: ${totalIcons}`);
	console.log(`📦 Total packages: ${Object.keys(manifest).length}`);
	console.log("\n💡 Next step:");
	console.log("   Run 'pnpm vendor' to generate icon files from this manifest");
	console.log();
}

// Run the script
main().catch((err) => {
	const message = err instanceof Error ? err.message : String(err);
	console.error("\n❌ Manifest generation failed:");
	error(message);
	if (err instanceof Error && err.stack) {
		console.error(err.stack);
	}
	process.exit(1);
});
