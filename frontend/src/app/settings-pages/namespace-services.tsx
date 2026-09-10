import { faExternalLink, faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import {
	Button,
	cn,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	Ping,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import {
	getProduct,
	getProductDocsUrl,
	ProductMark,
} from "@/components/products/product-picker";
import { SettingsCard } from "./settings-card";

// Services are Rivet-run workers the user connects to a namespace rather than
// code they deploy. Each one registers well-known actor names, so a namespace
// "has" the service once any of those names shows up in its builds. Setup
// instructions live in the docs so they are not duplicated here.
const SERVICES = [
	{
		product: getProduct("durable-streams"),
		actorNames: ["durableStream"],
	},
];

function openDocs(target: (typeof SERVICES)[number]["product"]["target"]) {
	window.open(getProductDocsUrl(target), "_blank", "noopener,noreferrer");
}

export function NamespaceServicesContent() {
	const dataProvider = useEngineCompatDataProvider();
	const { data: builds = [] } = useInfiniteQuery(
		dataProvider.buildsQueryOptions(),
	);

	return (
		<div className="space-y-4">
			<SettingsCard
				title="Services"
				description="Managed services connected to this namespace."
				divided
				action={
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button
								variant="outline"
								size="sm"
								startIcon={
									<Icon icon={faPlus} className="size-3" />
								}
							>
								Add Service
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent className="min-w-[--radix-popper-anchor-width]">
							{SERVICES.map((service) => (
								<DropdownMenuItem
									key={service.product.target}
									indicator={
										<ProductMark
											fileName={
												service.product.markFileName
											}
											section={service.product.section}
											size="sm"
										/>
									}
									onSelect={() =>
										openDocs(service.product.target)
									}
								>
									{service.product.label}
								</DropdownMenuItem>
							))}
						</DropdownMenuContent>
					</DropdownMenu>
				}
			>
				{SERVICES.map((service, idx) => {
					const connected = builds.some((build) =>
						service.actorNames.includes(build.id),
					);
					return (
						<div
							key={service.product.target}
							className={cn(
								"flex items-center justify-between gap-4 px-5 py-3 text-sm",
								idx < SERVICES.length - 1 &&
									"border-b border-foreground/10",
							)}
						>
							<div className="flex items-center gap-3 min-w-0">
								<ProductMark
									fileName={service.product.markFileName}
									section={service.product.section}
								/>
								<div className="min-w-0">
									<div className="text-foreground">
										{service.product.label}
									</div>
									<div className="text-xs text-muted-foreground truncate">
										{service.product.description}
									</div>
								</div>
							</div>
							<div className="flex items-center gap-3 shrink-0">
								{connected ? (
									<span className="inline-flex items-center gap-2 text-xs text-muted-foreground">
										<Ping
											variant="success"
											className="relative left-0 right-0 top-0"
										/>
										Connected
									</span>
								) : (
									<span className="text-xs text-muted-foreground">
										Not connected
									</span>
								)}
								<Button
									variant="outline"
									size="sm"
									endIcon={
										<Icon
											icon={faExternalLink}
											className="size-3"
										/>
									}
									onClick={() =>
										openDocs(service.product.target)
									}
								>
									{connected ? "Docs" : "Set up"}
								</Button>
							</div>
						</div>
					);
				})}
			</SettingsCard>
		</div>
	);
}
