import {
	faAws,
	faCloudflare,
	faGoogleCloud,
	faHetznerH,
	faKubernetes,
	faRailway,
	faServer,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import { Badge, Button } from "@/components";

const PROVIDERS = [
	{
		title: "Serverless",
		items: [
			{
				title: "Vercel",
				icon: faVercel,
				slug: "vercel",
				type: "1-click-deploy",
			},
			{
				title: "Cloudflare Workers",
				icon: faCloudflare,
				slug: "cloudflare",
			},
		],
	},
	{
		title: "Containers",
		items: [
			{
				title: "Railway",
				icon: faRailway,
				slug: "railway",
				type: "1-click-deploy",
			},
			{ title: "Kubernetes", icon: faKubernetes, slug: "kubernetes" },
			{ title: "AWS ECS", icon: faAws, slug: "aws-ecs" },
			{
				title: "Google Cloud Run",
				icon: faGoogleCloud,
				slug: "gcp-cloud-run",
			},
		],
	},
	{
		title: "Virtual Machines",
		items: [
			{ title: "Hetzner", icon: faHetznerH, slug: "hetzner" },
			{ title: "VM & Bare Metal", icon: faServer, slug: "vm-bare-metal" },
		],
	},
] as const;

export type Provider = (typeof PROVIDERS)[number]["items"][number]["slug"];

export function TemplateProviders({
	onProviderSelect,
}: {
	onProviderSelect?: (providerSlug: Provider) => void;
}) {
	return (
		<div className="grid grid-cols-1 gap-6 sm:grid-cols-1">
			{PROVIDERS.map((group) => (
				<div key={group.title}>
					<h3 className="mb-2 text-sm font-medium text-muted-foreground">
						{group.title}
					</h3>
					<div className="flex flex-col gap-2">
						{group.items.map((provider) => (
							<Button
								onClick={() =>
									onProviderSelect?.(provider.slug)
								}
								key={provider.slug}
								variant="outline"
								className="flex gap-3 justify-start "
							>
								<div className="text-xl text-foreground">
									<Icon icon={provider.icon} />
								</div>
								<div className="text-sm font-medium text-foreground">
									{provider.title}
								</div>
								{"type" in provider &&
									provider.type === "1-click-deploy" && (
										<Badge variant="secondary">
											1-Click Deploy
										</Badge>
									)}
							</Button>
						))}
					</div>
				</div>
			))}
		</div>
	);
}
