import {
	faCircle,
	faEllipsisVertical,
	faPlus,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Button,
	cn,
} from "@/components";

// -- Providers --

interface ProviderRow {
	name: string;
	provider: string;
	providerIcon: typeof faVercel;
	endpoint: string;
	region: string;
	regionFlag: string;
}

const PROVIDERS: ProviderRow[] = [
	{
		name: "default",
		provider: "Vercel",
		providerIcon: faVercel,
		endpoint: "https://rivetkit-example-vercel.app/api/rivet",
		region: "Northern Virginia, USA",
		regionFlag: "🇺🇸",
	},
];

function SettingsCard({
	title,
	description,
	action,
	children,
}: {
	title: string;
	description: string;
	action?: React.ReactNode;
	children: React.ReactNode;
}) {
	return (
		<div className="rounded-xl border dark:border-white/10 bg-card overflow-hidden">
			<div className="flex items-start justify-between gap-4 px-6 pt-5 pb-4">
				<div>
					<h3 className="text-base font-semibold text-foreground">
						{title}
					</h3>
					<p className="mt-0.5 text-xs text-muted-foreground">
						{description}
					</p>
				</div>
				{action}
			</div>
			{children}
		</div>
	);
}

function ProvidersSection() {
	return (
		<SettingsCard
			title="Providers"
			description="Clouds connected to Rivet for running Rivet Actors."
			action={
				<Button
					variant="outline"
					size="sm"
					className="gap-1.5 shrink-0"
				>
					<Icon icon={faPlus} className="w-3" />
					Add Provider
				</Button>
			}
		>
			<div className="border-t dark:border-white/10">
				<div
					className={cn(
						"grid grid-cols-[auto_1.2fr_1fr_2fr_1.4fr_auto] items-center gap-4 px-6 py-3",
						"text-[11px] font-medium uppercase tracking-wider text-muted-foreground",
						"border-b dark:border-white/10 bg-muted/20",
					)}
				>
					<span className="w-2" />
					<span>Name</span>
					<span>Provider</span>
					<span>Endpoint</span>
					<span>Datacenter</span>
					<span className="w-6" />
				</div>
				{PROVIDERS.map((p) => (
					<div
						key={p.name}
						className="grid grid-cols-[auto_1.2fr_1fr_2fr_1.4fr_auto] items-center gap-4 px-6 py-3.5 text-sm hover:bg-muted/20 transition-colors"
					>
						<Icon
							icon={faCircle}
							className="w-2 text-green-500"
						/>
						<span className="text-foreground font-medium truncate">
							{p.name}
						</span>
						<span className="flex items-center gap-2 text-foreground">
							<Icon
								icon={p.providerIcon}
								className="text-muted-foreground"
							/>
							{p.provider}
						</span>
						<span className="text-muted-foreground font-mono text-xs truncate">
							{p.endpoint}
						</span>
						<span className="flex items-center gap-2 text-foreground">
							<span className="text-base leading-none">
								{p.regionFlag}
							</span>
							<span className="truncate">{p.region}</span>
						</span>
						<button
							type="button"
							className="text-muted-foreground hover:text-foreground rounded p-1 -m-1"
							aria-label="Options"
						>
							<Icon
								icon={faEllipsisVertical}
								className="w-3.5"
							/>
						</button>
					</div>
				))}
			</div>
		</SettingsCard>
	);
}

// -- Runners --

function RunnersSection() {
	return (
		<SettingsCard
			title="Runners"
			description="Processes connected to Rivet Cloud and ready to start running Rivet Actors."
		>
			<div className="border-t dark:border-white/10">
				<div
					className={cn(
						"grid grid-cols-5 gap-4 px-6 py-3",
						"text-[11px] font-medium uppercase tracking-wider text-muted-foreground",
						"border-b dark:border-white/10 bg-muted/20",
					)}
				>
					<span>ID</span>
					<span>Name</span>
					<span>Datacenter</span>
					<span>Slots</span>
					<span>Version</span>
				</div>
				<div className="px-6 py-12 text-center">
					<p className="text-sm text-muted-foreground">
						Runners will be created when an actor is created.
					</p>
				</div>
			</div>
		</SettingsCard>
	);
}

// -- Advanced --

function TokenRow({
	label,
	value,
	description,
}: {
	label: string;
	value: string;
	description: string;
}) {
	return (
		<div className="space-y-1.5">
			<div className="flex items-baseline justify-between gap-2">
				<h4 className="text-sm font-medium text-foreground">{label}</h4>
				<Button variant="outline" size="sm" className="h-7 text-xs">
					Copy
				</Button>
			</div>
			<p className="text-xs text-muted-foreground">{description}</p>
			<div className="rounded-md border dark:border-white/10 bg-muted/30 px-3 py-2 font-mono text-xs text-muted-foreground tabular-nums truncate">
				{value}
			</div>
		</div>
	);
}

function AdvancedSection() {
	return (
		<Accordion type="single" collapsible>
			<AccordionItem
				value="advanced"
				className="rounded-xl border dark:border-white/10 bg-card overflow-hidden"
			>
				<AccordionTrigger className="px-6 py-4 hover:no-underline [&[data-state=open]]:border-b [&[data-state=open]]:dark:border-white/10">
					<span className="text-sm font-semibold text-foreground">
						Advanced
					</span>
				</AccordionTrigger>
				<AccordionContent className="px-6 py-5 space-y-6">
					<TokenRow
						label="Secret token"
						description="Server-only token used to authenticate with Rivet."
						value="sk_••••••••••••••••••••••••••••••••"
					/>
					<TokenRow
						label="Publishable token"
						description="Safe to use in clients and browsers."
						value="pk_live_91f3c2a7b4d5e6f7a8b9c0d1e2f3a4b5"
					/>
					<div className="pt-4 border-t dark:border-white/10">
						<h4 className="text-sm font-medium text-destructive">
							Danger zone
						</h4>
						<p className="mt-1 text-xs text-muted-foreground">
							Archive this namespace. This cannot be undone.
						</p>
						<Button
							variant="destructive"
							size="sm"
							className="mt-3"
						>
							Archive namespace
						</Button>
					</div>
				</AccordionContent>
			</AccordionItem>
		</Accordion>
	);
}

// -- Root --

export function SettingsContent() {
	return (
		<div className="space-y-6 pb-10">
			<ProvidersSection />
			<RunnersSection />
			<AdvancedSection />
		</div>
	);
}
