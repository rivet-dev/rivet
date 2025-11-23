import {
	faAws,
	faCloudflare,
	faGoogleCloud,
	faHetzner,
	faKubernetes,
	faRailway,
	faServer,
	faVercel,
} from "@rivet-gg/icons";

export interface DeployOption {
	title: string;
	href: string;
	description: string;
	icon?: any;
	badge?: string;
	/** If true, this platform should NOT be shown for generic deploy guides for Node/Bun-specific platforms. */
	specializedPlatform?: boolean;
}

export interface DeployGroup {
	title: string;
	items: DeployOption[];
}

export const deployGroups: DeployGroup[] = [
	{
		title: "Serverless",
		items: [
			{
				title: "Vercel",
				href: "/docs/connect/vercel",
				description:
					"Deploy Next.js + RivetKit apps to Vercel's edge network",
				icon: faVercel,
				badge: "1-Click Deploy",
				specializedPlatform: true,
			},
			{
				title: "Cloudflare Workers",
				href: "/docs/connect/cloudflare-workers",
				description:
					"Run your app on Cloudflare's global edge network with Durable Objects",
				icon: faCloudflare,
				specializedPlatform: true,
			},
		],
	},
	{
		title: "Containers",
		items: [
			{
				title: "Railway",
				href: "/docs/connect/railway",
				description:
					"Deploy containers to Railway's managed infrastructure",
				icon: faRailway,
				badge: "1-Click Deploy",
			},
			{
				title: "Kubernetes",
				href: "/docs/connect/kubernetes",
				description:
					"Deploy to any Kubernetes cluster with container images",
				icon: faKubernetes,
			},
			{
				title: "AWS ECS",
				href: "/docs/connect/aws-ecs",
				description:
					"Run containerized workloads on Amazon Elastic Container Service",
				icon: faAws,
			},
			{
				title: "Google Cloud Run",
				href: "/docs/connect/gcp-cloud-run",
				description:
					"Deploy containers to Google Cloud Run for auto-scaling",
				icon: faGoogleCloud,
			},
		],
	},
	{
		title: "Virtual Machines",
		items: [
			{
				title: "Hetzner",
				href: "/docs/connect/hetzner",
				description:
					"Deploy to Hetzner's cost-effective cloud infrastructure",
				icon: faHetzner,
			},
			{
				title: "VM & Bare Metal",
				href: "/docs/connect/vm-and-bare-metal",
				description:
					"Run on virtual machines or bare metal servers with full control",
				icon: faServer,
			},
		],
	},
];

// Flat list of all deploy options for backward compatibility
export const deployOptions: DeployOption[] = deployGroups.flatMap(
	(group) => group.items,
);
