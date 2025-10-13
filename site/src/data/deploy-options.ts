import {
	faVercel,
	faRailway,
	faDocker,
	faGoogleCloud,
	faAws,
	faCloudflare,
	faServer,
	faHetzner,
	faKubernetes,
} from "@rivet-gg/icons";

export interface DeployOption {
	title: string;
	href: string;
	icon?: any;
	badge?: string;
}

export const deployOptions: DeployOption[] = [
	{
		title: "Vercel",
		href: "/docs/deploy/vercel",
		icon: faVercel,
		badge: "1-Click Deploy",
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
