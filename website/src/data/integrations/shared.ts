export interface Integration {
	title: string;
	href: string;
	description: string;
}

export interface IntegrationGroup {
	title: string;
	items: Integration[];
}

export const integrationGroups: IntegrationGroup[] = [
	{
		title: "Backend",
		items: [
			{
				title: "Hono",
				href: "/docs/integrations/hono",
				description:
					"Lightweight and fast web framework for modern JavaScript",
			},
			{
				title: "Express",
				href: "/docs/integrations/express",
				description:
					"Popular Node.js web framework with extensive middleware support",
			},
			{
				title: "Elysia",
				href: "/docs/integrations/elysia",
				description: "Fast and type-safe TypeScript web framework",
			},
			{
				title: "tRPC",
				href: "/docs/integrations/trpc",
				description: "End-to-end type-safe API development",
			},
			{
				title: "Next.js",
				href: "/docs/integrations/next-js",
				description:
					"Full-stack React framework that brings Rivet into the Next.js runtime",
			},
		],
	},
	// {
	// 	title: "Auth",
	// 	items: [
	// 		{
	// 			title: "Better Auth",
	// 			href: "/docs/integrations/better-auth",
	// 			description:
	// 				"Modern authentication library with TypeScript support",
	// 		},
	// 	],
	// },
	{
		title: "Misc",
		items: [
			{
				title: "Vitest",
				href: "/docs/integrations/vitest",
				description:
					"Fast unit testing framework for JavaScript and TypeScript",
			},
			{
				title: "Pino",
				href: "/docs/integrations/pino",
				description:
					"High-performance structured logger for Node.js services",
			},
		],
	},
];
