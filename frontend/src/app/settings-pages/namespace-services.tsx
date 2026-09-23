import { faExternalLink, faPlus, Icon } from "@rivet-gg/icons";
import { useQueries, useQuery } from "@tanstack/react-query";
import {
	Button,
	cn,
	DiscreteCopyButton,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	Ping,
	Skeleton,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import {
	getProduct,
	getProductDocsUrl,
	ProductMark,
} from "@/components/products/product-picker";
import { features } from "@/lib/features";
import { getRivetRunUrl } from "@/lib/env";
import {
	MANAGED_SERVICES_POOL,
	MANAGED_SERVICES_POOL_CONFIG,
	useEnableManagedServicesMutation,
	useManagedServicesPoolQueryOptions,
} from "../managed-services";
import { SettingsCard } from "./settings-card";

// Services are Rivet-run workers the user connects to a namespace rather than
// code they deploy. Each one registers well-known actor names, so a namespace
// "has" the service once an actor under any of those names exists. Setup
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
	return features.compute ? <CloudServices /> : <ServicesList />;
}

// On Rivet Cloud every managed service runs in one dedicated compute pool, so
// the namespace has to provision it before any service is reachable. Other
// flavors run the service worker themselves and have nothing to enable.
function CloudServices() {
	const { data: pool, isLoading } = useQuery(
		useManagedServicesPoolQueryOptions(),
	);

	if (isLoading) {
		return <ServicesSkeleton />;
	}
	if (!pool) {
		return <EnableServices />;
	}
	return <ServicesList />;
}

// Mirrors one row of ServicesList. The card chrome is static copy, so only the
// per-service parts shimmer.
function ServicesSkeleton() {
	return (
		<div className="space-y-4">
			<SettingsCard
				title="Services"
				description="Managed services connected to this namespace."
				divided
				action={<Skeleton className="h-8 w-28" />}
			>
				<div className="flex items-center justify-between gap-4 px-5 py-3">
					<div className="flex items-center gap-3 min-w-0">
						<Skeleton className="size-8 rounded-xl" />
						<div className="min-w-0 space-y-1.5">
							<Skeleton className="h-4 w-32" />
							<Skeleton className="h-3 w-56" />
						</div>
					</div>
					<div className="flex items-center gap-3 shrink-0">
						<Skeleton className="h-3 w-20" />
						<Skeleton className="h-8 w-16" />
					</div>
				</div>
			</SettingsCard>
		</div>
	);
}

function EnableServices() {
	const { mutate, isPending } = useEnableManagedServicesMutation();

	return (
		<div className="flex flex-col items-center gap-3 rounded-md border border-dashed bg-card/50 px-6 py-10 text-center">
			<div className="text-sm text-foreground">
				Services are not enabled
			</div>
			<p className="text-xs text-muted-foreground max-w-md">
				Enable services to provision the managed pool that runs them in
				this namespace.
			</p>
			<Button
				size="sm"
				isLoading={isPending}
				onClick={() => mutate(MANAGED_SERVICES_POOL_CONFIG)}
			>
				Enable services
			</Button>
		</div>
	);
}

function ServicesList() {
	const dataProvider = useEngineCompatDataProvider();
	// On Rivet Cloud every managed service is served under the namespace's Rivet
	// Run origin. Self-hosted flavors run the worker themselves, so there is no
	// fixed URL to hand out.
	const rivetRunUrl =
		features.compute && features.services
			? getRivetRunUrl(
					dataProvider.engineNamespace,
					MANAGED_SERVICES_POOL,
				)
			: null;
	// Probed per service rather than read off the namespace build list: that
	// list is paginated, so a service registered past the first page would read
	// as not connected.
	const connections = useQueries({
		queries: SERVICES.map((service) => ({
			...dataProvider.actorsListPage1PollQueryOptions({
				n: service.actorNames,
				filters: { showDestroyed: { value: ["true"] } },
			}),
			select: (data: { actors: unknown[] }) => data.actors.length > 0,
		})),
	});

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
					const connected = connections[idx]?.data ?? false;
					const serviceUrl = rivetRunUrl
						? `${rivetRunUrl.replace(/\/?$/, "/")}${service.product.target}/`
						: null;
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
									{serviceUrl ? (
										<DiscreteCopyButton
											value={serviceUrl}
											className="mt-1 max-w-full font-mono text-xs text-muted-foreground"
										>
											{serviceUrl}
										</DiscreteCopyButton>
									) : null}
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
