import {
    faAws,
    faCloudflare,
    faGoogleCloud,
    faHetznerH,
    faKubernetes,
    faNetlify,
    faRailway,
    faRocket,
    faServer,
    faVercel,
} from "@rivet-gg/icons";


export interface DeployOption {
	displayName: string;
	name: string;
	shortTitle?: string;
	href: string;
	description: string;
	icon?: any;
	badge?: string;
	/** If true, this platform should NOT be shown for generic deploy guides for Node/Bun-specific platforms. */
	specializedPlatform?: boolean;
}

export const deployOptions = [
	{
		displayName: "Vercel",
		name: "vercel" as const,
		href: "/docs/connect/vercel",
		description:
			"Deploy Next.js + RivetKit apps to Vercel's edge network",
		icon: faVercel as any,
		badge: "1-Click Deploy",
	},
	{
		displayName: "Netlify",
		name: "netlify" as const,
		href: "/docs/connect/netlify",
		description:
			"Deploy RivetKit apps to Netlify Functions with JAMstack",
		icon: faNetlify as any,
		badge: "1-Click Deploy",
	},
	{
		displayName: "Railway",
		name: "railway" as const,
		href: "/docs/connect/railway",
		description:
			"Deploy containers to Railway's managed infrastructure",
		icon: faRailway as any,
		badge: "1-Click Deploy",
	},
	{
		displayName: "Cloudflare Workers",
		name: "cloudflare-workers" as const,
		shortTitle: "Cloudflare",
		href: "/docs/connect/cloudflare-workers",
		description:
			"Run your app on Cloudflare's global edge network with Durable Objects",
		icon: faCloudflare as any,
		specializedPlatform: true,
	},
	{
		displayName: "Kubernetes",
		name: "kubernetes" as const,
		href: "/docs/connect/kubernetes",
		description:
			"Deploy to any Kubernetes cluster with container images",
		icon: faKubernetes as any,
	},
	{
		displayName: "AWS ECS",
		shortTitle: "AWS",
		name: "aws-ecs" as const,
		href: "/docs/connect/aws-ecs",
		description:
			"Run containerized workloads on Amazon Elastic Container Service",
		icon: faAws as any,
	},
	{
		displayName: "Google Cloud Run",
		shortTitle: "GCP",
		name: "gcp-cloud-run" as const,
		href: "/docs/connect/gcp-cloud-run",
		description:
			"Deploy containers to Google Cloud Run for auto-scaling",
		icon: faGoogleCloud,
	},
	{
		displayName: "Hetzner",
		name: "hetzner" as const,
		href: "/docs/connect/hetzner",
		description:
			"Deploy to Hetzner's cost-effective cloud infrastructure",
		icon: faHetznerH as any,
	},
	{
		displayName: "VM & Bare Metal",
		name: "custom" as const,
		shortTitle: "VM",
		href: "/docs/connect/vm-and-bare-metal",
		description:
			"Run on virtual machines or bare metal servers with full control",
		icon: faServer as any,
	},
	{
		displayName: "Custom Platform",
		name: "custom-platform" as const,
		href: "/docs/connect/custom",
		description:
			"Integrate RivetKit with any other hosting platform of your choice",
		icon: faRocket as any,
	}
] satisfies DeployOption[];


export type Provider = typeof deployOptions[number]["name"];