const CATEGORIES = [
	{
		name: "Multiplayer",
		description:
			"Engage players with live multiplayer gameplay, fostering competition and cooperation.",
		slug: "multiplayer",
	},
	{
		name: "Authentication",
		description:
			"Secure and manage user accounts to personalize and safeguard the player experience.",
		slug: "auth",
	},
	{
		name: "Social",
		description:
			"Facilitate player interaction and community-building to boost engagement and retention.",
		slug: "social",
	},
	{
		name: "Economy",
		description:
			"Drive player progression and monetization with virtual goods and currencies.",
		slug: "economy",
	},
	// {
	//   name: "Monetization",
	//   description: "TODO",
	//   slug: "monetization",
	// },
	{
		name: "Competitive",
		description:
			"Motivate and reward skilled play with rankings, tournaments, and leagues.",
		slug: "competitive",
	},
	{
		name: "Analytics",
		description:
			"Gain actionable insights to optimize game design, balance, and monetization.",
		slug: "analytics",
	},
	// {
	//   name: "Monitoring",
	//   description: "TODO",
	//   slug: "monitoring",
	// },
	{
		name: "Security",
		description:
			"Protect your game and players from cheating, hacking, and disruptive behavior.",
		slug: "security",
	},
	{
		name: "Utility",
		description:
			"Streamline development with foundational tools and reusable components.",
		slug: "utility",
	},
	{
		name: "Platform",
		description:
			"Extend your game's reach and engage players across popular gaming platforms.",
		slug: "platform",
	},
	// {
	//   name: "Infrastructure",
	//   description: "Extend and integrate your game with custom backend services and third-party APIs.",
	//   slug: "infra",
	// },
	{
		name: "Service",
		description:
			"Integrate third-party services to enhance functionality and streamline operations.",
		slug: "service",
	},
];

export async function loadModulesMeta() {
	const versionMetaResponse = await fetch(
		"https://releases.rivet.gg/backend/index.json",
	);
	const { latestVersion } = await versionMetaResponse.json();
	const modulesMetaResponse = await fetch(
		`https://releases.rivet.gg/backend/${latestVersion}/index.json`,
	);
	return await modulesMetaResponse.json();
}

export async function loadModuleMeta(module: string) {
	const meta = await loadModulesMeta();
	const moduleMeta = meta.modules[module];
	return {
		...moduleMeta,
		category: CATEGORIES.find(
			(category) => moduleMeta.config.tags.indexOf(category.slug) !== -1,
		) ?? { name: "Uncategorized", slug: "uncategorized", description: "" },
		config: {
			...moduleMeta.config,
			dependencies: Object.fromEntries(
				Object.keys(moduleMeta.config.dependencies || {}).map((dep) => [
					dep,
					meta.modules[dep],
				]),
			),
		},
	};
}

export interface Category {
	name: string;
	slug: string;
	description: string;
	modules: {
		id: string;
		module: {
			config: {
				status: string;
				name: string;
				description: string;
				icon: string;
			};
		};
	}[];
}

export async function loadModuleCategories() {
	const meta = await loadModulesMeta();
	const unsortedModules = new Set(Object.keys(meta.modules));
	const allCategories: Category[] = [];
	for (const categoryConfig of CATEGORIES) {
		const category: Category = {
			...categoryConfig,
			modules: [],
		};
		allCategories.push(category);

		// Find modules
		for (const moduleId of new Set(unsortedModules)) {
			const mod = meta.modules[moduleId];
			if (mod.config.tags?.indexOf("internal") !== -1) {
				unsortedModules.delete(moduleId);
				continue;
			}

			if (mod.config.tags.indexOf(category.slug) === -1) continue;

			// Add to category
			unsortedModules.delete(moduleId);
			category.modules.push({ id: moduleId, module: mod });
		}

		// Sort modules
		category.modules = category.modules.sort((a, b) => {
			// Sink 'coming_soon' modules to the bottom
			if (
				a.module.config.status === "coming_soon" &&
				b.module.config.status !== "coming_soon"
			)
				return 1;
			if (
				a.module.config.status !== "coming_soon" &&
				b.module.config.status === "coming_soon"
			)
				return -1;
			// For modules with the same status, sort alphabetically
			return a.module.config.name.localeCompare(b.module.config.name);
		});
	}

	// Check for unsorted modules
	if (unsortedModules.size !== 0) {
		throw new Error(
			`Modules do no have tag matching a category: ${[...unsortedModules].join(", ")}`,
		);
	}

	return allCategories;
}
