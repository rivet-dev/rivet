#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read --allow-write

// Import necessary Deno modules
import { join } from "https://deno.land/std@0.114.0/path/mod.ts";

// Define the directory path
const packagesDir = join(import.meta.dirname, "../../packages");

// Function to update Cargo.toml files recursively
async function updateCargoTomlFiles(dir: string) {
	// Read the directory
	for await (const dirEntry of Deno.readDir(dir)) {
		const fullPath = join(dir, dirEntry.name);

		if (dirEntry.isDirectory) {
			// Recursively update Cargo.toml files in subdirectories
			await updateCargoTomlFiles(fullPath);
		} else if (dirEntry.isFile && dirEntry.name === "Cargo.toml") {
			try {
				// Read the Cargo.toml file
				let data = await Deno.readTextFile(fullPath);

				// // Remove existing version, license, authors, and edition fields
				// data = data.replace(/^version\s*=\s*".*"\s*(\n|$)/gm, "");
				// data = data.replace(/^license\s*=\s*".*"\s*(\n|$)/gm, "");
				// data = data.replace(/^authors\s*=\s*\[.*\]\s*(\n|$)/gm, "");
				// data = data.replace(/^edition\s*=\s*".*"\s*(\n|$)/gm, "");

				// Find the end of the [package] block, ignore trailing whitespace, and insert new fields
				data = data.replace(
					/(\[package\][\s\S]*?)(\s*)(?=\n\[.*\]|\n?$)/,
					`$1\nversion.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nedition.workspace = true\n`,
				);

				// Write the updated Cargo.toml file
				console.log(data);
				await Deno.writeTextFile(fullPath, data);
				console.log(`Updated ${fullPath}`);
			} catch (error) {
				console.error(`Failed to update ${fullPath}:`, error);
			}
		}
	}
}

// Execute the function starting from the root packages directory
updateCargoTomlFiles(packagesDir);
