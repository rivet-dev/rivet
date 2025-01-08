// @ts-check
const fs = require("node:fs");
const { getPackageInfo, importModule, resolveModule } = require("local-pkg");
const { join } = require("node:path");

const icons = new Set();

const searchPaths = [
	join(__dirname, "..", "src", "node_modules"),
	join(__dirname, "..", "..", "..", "node_modules"),
];

function faCamelCase(str) {
	const [firstLetter, ...restLetters] = str.replace(/-./g, (g) =>
		g[1].toUpperCase(),
	);
	return `fa${[firstLetter.toUpperCase(), ...restLetters].join("")}`;
}

async function registerIcons(iconModuleName) {
	const info = await getPackageInfo(iconModuleName, {
		paths: searchPaths,
	});

	if (!info) {
		throw new Error(`Could not find package ${iconModuleName}`);
	}

	const { rootPath } = info;

	const module = resolveModule(iconModuleName, { paths: [rootPath] });
	if (!module) {
		throw new Error(`Could not resolve module ${iconModuleName}`);
	}
	const files = await fs.promises.readdir(rootPath);

	const iconFiles = files.filter(
		(file) => file.startsWith("fa") && file.endsWith(".js"),
	);

	const iconsModule = await importModule(module);

	const foundIcons = [];

	for (const iconFile of iconFiles) {
		const iconName = iconFile.replace(".js", "");
		const iconDefinition = iconsModule[iconName];

		const aliases = iconDefinition.icon?.[2].filter(
			(alias) => typeof alias === "string",
		);

		if (
			icons.has(iconDefinition.iconName) ||
			aliases.some((alias) => icons.has(alias))
		) {
			continue;
		}

		foundIcons.push({ icon: iconName, aliases: aliases.map(faCamelCase) });
	}

	return {
		[iconModuleName]: { icons: foundIcons, prefix: iconsModule.prefix },
	};
}

function registerCustomIcons(iconKit) {
	const module = require.resolve(iconKit, { paths: searchPaths });

	if (!module) {
		throw new Error(`Could not resolve module ${iconKit}`);
	}

	const customIcons = require(module);

	const foundIcons = [];

	for (const [iconName, iconDefinition] of Object.entries(customIcons)) {
		const aliases = iconDefinition.icon?.[2].filter(
			(alias) => typeof alias === "string",
		);

		if (
			icons.has(iconDefinition.iconName) ||
			aliases.some((alias) => icons.has(alias))
		) {
			continue;
		}

		foundIcons.push({ icon: iconName, aliases: aliases.map(faCamelCase) });
	}

	return { [iconKit]: { icons: foundIcons } };
}

async function generateManifest() {
	const manifest = {
		...(await registerIcons("@fortawesome/free-solid-svg-icons")),
		...(await registerIcons("@fortawesome/free-brands-svg-icons")),
		...(await registerIcons("@fortawesome/pro-solid-svg-icons")),
		...registerCustomIcons("@awesome.me/kit-63db24046b/icons/kit/custom"),
	};

	fs.writeFileSync(
		join(__dirname, "../manifest.json"),
		JSON.stringify(manifest),
	);
}

generateManifest().catch((e) => {
	console.error(e);
	process.exit(1);
});
