#!/usr/bin/env node
// @ts-nocheck
//
// RIVET ICONS SHARED UTILITIES
// Common utilities used by icon generation scripts
//

import fs from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import dedent from "dedent";

// ============================================================================
// CONSTANTS
// ============================================================================

const PATHS = {
	get root() {
		return join(import.meta.dirname, "..");
	},
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
	get dist() {
		return join(this.root, "dist");
	},
};

const FA_PACKAGES_CONFIG = {
	// Custom kit with Rivet-specific icons
	"@awesome.me/kit-63db24046b": "1.0.35",
	// Pro packages (regular and solid styles)
	"@fortawesome/pro-regular-svg-icons": "6.6.0",
	"@fortawesome/pro-solid-svg-icons": "6.6.0",
	// Free packages (used as fallbacks)
	"@fortawesome/free-solid-svg-icons": "7.2.0",
	"@fortawesome/free-brands-svg-icons": "7.2.0",
};

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
	console.error(`❌`, message);
}

/**
 * @param {string} message
 */
function exitWithError(message) {
	throw new Error(message);
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
// SETUP FUNCTIONS
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

function checkForCustomKitUpdates() {
	log("🔍", "Checking for custom kit updates...");

	const customKits = Object.entries(FA_PACKAGES_CONFIG).filter(([pkg]) =>
		pkg.startsWith("@awesome.me/"),
	);

	for (const [packageName, currentVersion] of customKits) {
		const result = spawnSync("npm", ["view", packageName, "version"], {
			cwd: PATHS.src,
			env: { ...process.env },
			encoding: "utf-8",
		});

		if (result.status !== 0 || !result.stdout) {
			error(`Failed to check latest version for ${packageName}`);
			continue;
		}

		const latestVersion = result.stdout.trim();
		if (latestVersion !== currentVersion) {
			console.log();
			console.log("⚠️  ========================================");
			console.log(`⚠️  UPDATE AVAILABLE: ${packageName}`);
			console.log(`⚠️  Current: ${currentVersion} → Latest: ${latestVersion}`);
			console.log(`⚠️  Update FA_PACKAGES_CONFIG in shared-utils.js`);
			console.log("⚠️  ========================================");
			console.log();
		} else {
			log("✅", `${packageName} is up to date (${currentVersion})`);
		}
	}
}

function installFontAwesomePackages() {
	log("📦", "Installing Font Awesome Pro packages...");
	log("⏳", "This may take a minute...");

	const result = spawnSync("npm", ["install", "--no-package-lock", "--force", "--silent"], {
		stdio: "inherit",
		cwd: PATHS.src,
		env: { ...process.env, CI: "0" },
	});

	if (result.status !== 0) {
		exitWithError("Failed to install Font Awesome packages");
	}

	log("✅", "Packages installed");
}

function cleanupGeneratedFiles() {
	log("🧹", "Cleaning up generated files...");

	const filesToRemove = [
		join(PATHS.src, ".npmrc"),
		join(PATHS.src, "package.json"),
	];

	for (const file of filesToRemove) {
		if (fs.existsSync(file)) {
			fs.unlinkSync(file);
		}
	}

	if (fs.existsSync(PATHS.srcNodeModules)) {
		fs.rmSync(PATHS.srcNodeModules, { recursive: true, force: true });
	}

	log("✅", "Cleanup complete");
}

// ============================================================================
// EXPORTS
// ============================================================================

export {
	PATHS,
	FA_PACKAGES_CONFIG,
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
};
