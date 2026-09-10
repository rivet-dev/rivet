import { faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { CONNECT_DURABLE_STREAMS_MODAL } from "@/app/dialogs/connect-provider-sheet";
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
import { getProduct, ProductMark } from "@/components/products/product-picker";
import { SettingsCard } from "./settings-card";

// Services are Rivet-run workers the user connects to a namespace rather than
// code they deploy. Each one registers well-known actor names, so a namespace
// "has" the service once any of those names shows up in its builds.
const SERVICES = [
	{
		product: getProduct("durable-streams"),
		modal: CONNECT_DURABLE_STREAMS_MODAL,
		actorNames: ["durableStream"],
	},
];

export function Services() {
	const navigate = useNavigate();
	const dataProvider = useEngineCompatDataProvider();
	const { data: builds = [] } = useInfiniteQuery(
		dataProvider.buildsQueryOptions(),
	);

	const openSetup = (modal: string) =>
		navigate({
			to: ".",
			search: (old) => ({
				...(old as Record<string, unknown>),
				modal,
			}),
		});

	return (
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
										fileName={service.product.markFileName}
										section={service.product.section}
										size="sm"
									/>
								}
								onSelect={() => openSetup(service.modal)}
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
								onClick={() => openSetup(service.modal)}
							>
								{connected ? "Setup" : "Connect"}
							</Button>
						</div>
					</div>
				);
			})}
		</SettingsCard>
	);
}
