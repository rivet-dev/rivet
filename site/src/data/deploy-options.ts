export interface DeployOption {
	title: string;
	href: string;
}

export const deployOptions: DeployOption[] = [
	{
		title: "Railway",
		href: "/docs/deploy/railway",
	},
	// {
	// 	title: "Vercel",
	// 	href: "/docs/deploy/vercel",
	// },
	// {
	// 	title: "Freestyle",
	// 	href: "/docs/deploy/freestyle",
	// },
	{
		title: "AWS ECS",
		href: "/docs/deploy/aws-ecs",
	},
	{
		title: "Google Cloud Run",
		href: "/docs/deploy/gcp-cloud-run",
	},
	{
		title: "Kubernetes",
		href: "/docs/deploy/kubernetes",
	},
	{
		title: "Hetzner",
		href: "/docs/deploy/hetzner",
	},
	{
		title: "VM & Bare Metal",
		href: "/docs/deploy/vm-and-bare-metal",
	},
	{
		title: "Cloudflare Workers",
		href: "/docs/deploy/cloudflare-workers",
	},
];
