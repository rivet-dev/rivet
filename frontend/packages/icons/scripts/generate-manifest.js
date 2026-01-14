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

import fs from "node:fs";
import { join } from "node:path";
import { getPackageInfo, importModule, resolveModule } from "local-pkg";
import {
	PATHS,
	log,
	error,
	exitWithError,
	faCamelCase,
	checkEnvironment,
	setupSourceDirectory,
	configureFontAwesomeRegistry,
	checkForCustomKitUpdates,
	installFontAwesomePackages,
	cleanupGeneratedFiles,
} from "./shared-utils.js";

// ============================================================================
// CONFIGURATION
// ============================================================================

const SEARCH_PATHS = [PATHS.srcNodeModules, PATHS.rootNodeModules];

const FA_PACKAGES = [
	"@fortawesome/free-solid-svg-icons",
	"@fortawesome/free-brands-svg-icons",
	"@fortawesome/pro-solid-svg-icons",
];

const CUSTOM_KITS = ["@awesome.me/kit-63db24046b/icons/kit/custom"];

// Legacy kit paths use the old JS format (not ES modules)
const LEGACY_CUSTOM_KITS = [
	"@awesome.me/kit-63db24046b/icons/js/custom-icons-duotone.js",
];

// Track globally registered icons to avoid duplicates
const registeredIcons = new Set();

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
async function registerCustomKit(kitPath) {
	log("📦", `Processing custom kit: ${kitPath}...`);

	// Resolve the custom kit module
	const modulePath = resolveModule(kitPath, { paths: SEARCH_PATHS });

	if (!modulePath) {
		throw new Error(`Could not resolve custom kit: ${kitPath}`);
	}

	// Load the custom icons
	const customIcons = await importModule(modulePath);
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

/**
 * Registers icons from a legacy Font Awesome kit JS file
 * @param {string} kitPath - Path to the legacy kit JS file
 * @returns {Promise<Record<string, {icons: Array}>>}
 */
async function registerLegacyCustomKit(kitPath) {
	log("📦", `Processing legacy kit: ${kitPath}...`);

	// Find the package in node_modules
	const jsFilePath = join(PATHS.srcNodeModules, kitPath);

	if (!fs.existsSync(jsFilePath)) {
		throw new Error(`Could not find legacy kit file: ${jsFilePath}`);
	}

	// Read and parse the JS file to extract icon definitions
	const jsContent = fs.readFileSync(jsFilePath, "utf-8");

	// The legacy format has icons defined as: "iconName": [width, height, aliases, unicode, svgPathData]
	// We need to extract icon names from this format
	const iconRegex = /"([a-z0-9-]+)":\s*\[\d+,\s*\d+,\s*\[[^\]]*\],\s*"[^"]+"/g;
	const foundIcons = [];
	let skippedCount = 0;
	let match;

	while ((match = iconRegex.exec(jsContent)) !== null) {
		const iconBaseName = match[1];

		// Skip if already registered
		if (registeredIcons.has(iconBaseName)) {
			skippedCount++;
			continue;
		}

		const iconName = faCamelCase(iconBaseName);
		registeredIcons.add(iconBaseName);
		foundIcons.push({ icon: iconName, aliases: [iconName] });
	}

	log(
		"✅",
		`Found ${foundIcons.length} legacy icons${skippedCount > 0 ? ` (skipped ${skippedCount} duplicates)` : ""}`,
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

	// Check environment before creating any files
	checkEnvironment();

	// Setup and generate with guaranteed cleanup
	setupSourceDirectory();
	try {
		configureFontAwesomeRegistry();
		checkForCustomKitUpdates();
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
				const kitManifest = await registerCustomKit(kitPath);
				Object.assign(manifest, kitManifest);
				totalIcons += kitManifest[kitPath].icons.length;
			} catch (err) {
				const message = err instanceof Error ? err.message : String(err);
				error(`Failed to register custom kit ${kitPath}: ${message}`);
				throw err;
			}
		}

		// Register legacy custom kits (duotone, etc.)
		for (const kitPath of LEGACY_CUSTOM_KITS) {
			try {
				const kitManifest = await registerLegacyCustomKit(kitPath);
				Object.assign(manifest, kitManifest);
				totalIcons += kitManifest[kitPath].icons.length;
			} catch (err) {
				const message = err instanceof Error ? err.message : String(err);
				error(`Failed to register legacy kit ${kitPath}: ${message}`);
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
	} finally {
		console.log();
		cleanupGeneratedFiles();
	}
}

// Run the script
main().catch((err) => {
	console.error("\n❌ Manifest generation failed:");
	error(err);
	process.exit(1);
});
