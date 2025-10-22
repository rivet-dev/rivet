import {
	faAws,
	faCloudflare,
	faDocker,
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
	icon?: any;
	badge?: string;
	/** If true, this platform should NOT be shown for generic deploy guides for Node/Buns-specific platforms. */
	specializedPlatform?: boolean;
}

export const deployOptions: DeployOption[] = [
	{
		title: "Vercel",
		href: "/docs/deploy/vercel",
		icon: faVercel,
		badge: "1-Click Deploy",
		specializedPlatform: true,
	},
	{
		title: "Railway",
		href: "/docs/deploy/railway",
		icon: faRailway,
		badge: "1-Click Deploy",
	},
	{
		title: "Cloudflare Workers",
		href: "/docs/deploy/cloudflare-workers",
		icon: faCloudflare,
		specializedPlatform: true,
	},
	{
		title: "Kubernetes",
		href: "/docs/deploy/kubernetes",
		icon: faKubernetes,
	},
	{
		title: "AWS ECS",
		href: "/docs/deploy/aws-ecs",
		icon: faAws,
	},
	{
		title: "Google Cloud Run",
		href: "/docs/deploy/gcp-cloud-run",
		icon: faGoogleCloud,
	},
	// {
	// 	title: "Freestyle",
	// 	href: "/docs/deploy/freestyle",
	// specializedPlatform: true,
	// },
	{
		title: "Hetzner",
		href: "/docs/deploy/hetzner",
		icon: faHetzner,
	},
	{
		title: "VM & Bare Metal",
		href: "/docs/deploy/vm-and-bare-metal",
		icon: faServer,
	},
];
