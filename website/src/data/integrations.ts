import eve from "@/images/vendors/eve.svg";
import flue from "@/images/vendors/flue.svg";
import workflow from "@/images/vendors/workflow.svg";
import type { SidebarItem } from "@/lib/sitemap";

export const integrations = [
	{
		title: "Flue",
		description: "Run Flue agents on Rivet with agentOS sandboxes.",
		href: "/integrations/flue",
		category: "Agents",
		icon: { src: flue.src },
		badge: "Beta",
	},
	{
		title: "Vercel Eve",
		description: "Use Rivet as the durable World for Vercel Eve.",
		href: "/integrations/vercel-eve",
		category: "Agents",
		icon: { src: eve.src },
		badge: "Beta",
	},
	{
		title: "Vercel Workflows",
		description: "Vercel Workflows backed by Rivet Actors.",
		href: "/integrations/vercel-workflows",
		category: "Workflows",
		icon: { src: workflow.src },
		badge: "Beta",
	},
] as const;

const integrationCategories = ["Agents", "Workflows"] as const;

export const integrationSidebarSections = integrationCategories.map(
	(title) => ({
		title,
		pages: integrations
			.filter((integration) => integration.category === title)
			.map(({ title, href, icon, badge }) => ({ title, href, icon, badge })),
	}),
) satisfies SidebarItem[];
