import {
	faAws,
	faCheck,
	faCircle,
	faCopy,
	faEllipsisVertical,
	faGoogleCloud,
	faHetznerH,
	faPencil,
	faPlus,
	faRailway,
	faServer,
	faTrash,
	faVercel,
	Icon,
} from "@rivet-gg/icons";
import { useState } from "react";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Badge,
	Button,
	Checkbox,
	cn,
	Dialog,
	DialogContent,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
	Input,
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

const PROVIDER_OPTIONS: { name: string; icon: typeof faVercel }[] = [
	{ name: "Vercel", icon: faVercel },
	{ name: "Railway", icon: faRailway },
	{ name: "AWS ECS", icon: faAws },
	{ name: "Google Cloud Run", icon: faGoogleCloud },
	{ name: "Hetzner", icon: faHetznerH },
	{ name: "Custom", icon: faServer },
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

const PROVIDER_DATACENTERS: { name: string; flag: string }[] = [
	{ name: "Singapore", flag: "🇸🇬" },
	{ name: "Frankfurt, Germany", flag: "🇩🇪" },
	{ name: "Northern Virginia, USA", flag: "🇺🇸" },
	{ name: "Oregon, USA", flag: "🇺🇸" },
];

function Field({
	label,
	description,
	children,
}: {
	label: string;
	description?: string;
	children: React.ReactNode;
}) {
	return (
		<div>
			<label className="block text-xs font-medium text-foreground mb-1.5">
				{label}
			</label>
			{children}
			{description ? (
				<p className="mt-1 text-[11px] text-muted-foreground leading-relaxed">
					{description}
				</p>
			) : null}
		</div>
	);
}

function EditProviderDialog({
	provider,
	onOpenChange,
}: {
	provider: ProviderRow | null;
	onOpenChange: (open: boolean) => void;
}) {
	const [tab, setTab] = useState<"global" | "datacenter">("global");

	return (
		<Dialog open={!!provider} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-md p-0 gap-0 overflow-hidden">
				{provider ? (
					<>
						<div className="flex items-center justify-between px-5 pt-5 pb-4 pr-12">
							<h2 className="text-base font-semibold tracking-tight">
								Edit '{provider.name}' Provider
							</h2>
						</div>
						<div className="grid grid-cols-2 border-y dark:border-white/10">
							{[
								{ value: "global", label: "Global Settings" },
								{
									value: "datacenter",
									label: "Per Datacenter Settings",
								},
							].map((t) => (
								<button
									key={t.value}
									type="button"
									onClick={() =>
										setTab(
											t.value as
												| "global"
												| "datacenter",
										)
									}
									className={cn(
										"py-2.5 text-xs font-medium transition-colors border-b-2 -mb-px",
										tab === t.value
											? "border-b-foreground text-foreground"
											: "border-b-transparent text-muted-foreground hover:text-foreground",
									)}
								>
									{t.label}
								</button>
							))}
						</div>
						<div className="px-5 pt-4 pb-5 space-y-4 overflow-y-auto max-h-[calc(80vh-180px)]">
							<p className="text-xs text-muted-foreground">
								{tab === "global"
									? "These settings will apply to all datacenters."
									: "Override global settings for specific datacenters."}
							</p>
							<Field label="Endpoint">
								<Input
									defaultValue="https://lexus-gx550-tracker-production.up.railway.app/api/rivet"
									className="h-8 text-xs font-mono"
								/>
							</Field>
							<div className="grid grid-cols-2 gap-3">
								<Field
									label="Min Runners"
									description="The minimum number of runners to keep running."
								>
									<Input
										type="number"
										defaultValue={0}
										className="h-8 text-xs"
									/>
								</Field>
								<Field
									label="Max Runners"
									description="The maximum number of runners that can be created to handle load."
								>
									<Input
										type="number"
										defaultValue={10000}
										className="h-8 text-xs"
									/>
								</Field>
							</div>
							<div className="grid grid-cols-2 gap-3">
								<Field
									label="Request Lifespan"
									description="The maximum duration (in seconds) a request can take before being terminated."
								>
									<Input
										type="number"
										defaultValue={900}
										className="h-8 text-xs"
									/>
								</Field>
								<Field
									label="Slots Per Runner"
									description="The number of concurrent slots each runner can handle."
								>
									<Input
										type="number"
										defaultValue={1}
										className="h-8 text-xs"
									/>
								</Field>
							</div>
							<Field
								label="Runners Margin"
								description="The number of extra runners to keep running to handle sudden spikes in load."
							>
								<Input
									type="number"
									defaultValue={0}
									className="h-8 text-xs"
								/>
							</Field>
							<div>
								<label className="block text-xs font-medium text-foreground">
									Custom Headers
								</label>
								<p className="mt-1 text-[11px] text-muted-foreground leading-relaxed">
									Custom headers to add to each request to the
									runner. Useful for providing authentication
									or other information.
								</p>
								<Button
									variant="outline"
									size="sm"
									className="mt-2 h-7 gap-1.5 text-xs"
								>
									<Icon icon={faPlus} className="w-2.5" />
									Add a header
								</Button>
							</div>
							<div>
								<h4 className="text-xs font-medium text-foreground">
									Datacenters
								</h4>
								<p className="mt-1 text-[11px] text-muted-foreground leading-relaxed">
									Datacenters where this provider can deploy
									actors.
								</p>
								<div className="mt-2.5 space-y-2">
									{PROVIDER_DATACENTERS.map((dc) => (
										<label
											key={dc.name}
											className="flex items-center gap-2.5 cursor-pointer"
										>
											<Checkbox defaultChecked />
											<span className="text-sm leading-none">
												{dc.flag}
											</span>
											<span className="text-xs text-foreground">
												{dc.name}
											</span>
										</label>
									))}
								</div>
							</div>
						</div>
						<div className="flex justify-end gap-2 px-5 py-3 border-t dark:border-white/10 bg-muted/20">
							<Button
								variant="ghost"
								size="sm"
								className="h-8"
								onClick={() => onOpenChange(false)}
							>
								Cancel
							</Button>
							<Button
								size="sm"
								className="h-8 bg-foreground text-background hover:bg-foreground/90"
								onClick={() => onOpenChange(false)}
							>
								Save
							</Button>
						</div>
					</>
				) : null}
			</DialogContent>
		</Dialog>
	);
}

function ProvidersSection() {
	const [editingProvider, setEditingProvider] = useState<ProviderRow | null>(
		null,
	);

	return (
		<>
			<SettingsCard
				title="Providers"
				description="Clouds connected to Rivet for running Rivet Actors."
				action={
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button
								variant="outline"
								size="sm"
								className="gap-1.5 shrink-0"
							>
								<Icon icon={faPlus} className="w-3" />
								Add Provider
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align="end" className="w-52">
							{PROVIDER_OPTIONS.map((opt) => (
								<DropdownMenuItem
									key={opt.name}
									className="gap-2.5"
								>
									<Icon
										icon={opt.icon}
										className="w-3.5 text-muted-foreground"
									/>
									{opt.name}
								</DropdownMenuItem>
							))}
						</DropdownMenuContent>
					</DropdownMenu>
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
							<DropdownMenu>
								<DropdownMenuTrigger asChild>
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
								</DropdownMenuTrigger>
								<DropdownMenuContent
									align="end"
									className="w-36"
								>
									<DropdownMenuItem
										className="gap-2.5"
										onSelect={() => setEditingProvider(p)}
									>
										<Icon
											icon={faPencil}
											className="w-3.5 text-muted-foreground"
										/>
										Edit
									</DropdownMenuItem>
									<DropdownMenuItem className="gap-2.5 text-destructive focus:text-destructive">
										<Icon
											icon={faTrash}
											className="w-3.5"
										/>
										Remove
									</DropdownMenuItem>
								</DropdownMenuContent>
							</DropdownMenu>
						</div>
					))}
				</div>
			</SettingsCard>
			<EditProviderDialog
				provider={editingProvider}
				onOpenChange={(open) => {
					if (!open) setEditingProvider(null);
				}}
			/>
		</>
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

function SectionHeading({
	title,
	description,
	badge,
	action,
}: {
	title: string;
	description: string;
	badge?: string;
	action?: React.ReactNode;
}) {
	return (
		<div className="flex items-start justify-between gap-4">
			<div className="min-w-0">
				<div className="flex items-center gap-2">
					<h4 className="text-sm font-semibold text-foreground">
						{title}
					</h4>
					{badge ? (
						<Badge
							variant="outline"
							className="h-4 px-1.5 text-[10px] font-medium tracking-wide uppercase border-foreground/20"
						>
							{badge}
						</Badge>
					) : null}
				</div>
				<p className="mt-1 text-xs text-muted-foreground leading-relaxed">
					{description}
				</p>
			</div>
			{action}
		</div>
	);
}

function CopyFieldRow({
	label,
	value,
	mono = true,
}: { label: string; value: string; mono?: boolean }) {
	return (
		<div className="grid grid-cols-[170px_1fr_auto] items-center gap-3 px-3 py-2 border-b last:border-b-0 dark:border-white/10 text-xs hover:bg-muted/20 transition-colors">
			<span
				className={cn(
					"text-muted-foreground truncate",
					mono && "font-mono",
				)}
			>
				{label}
			</span>
			<span
				className={cn(
					"text-foreground truncate",
					mono && "font-mono tabular-nums",
				)}
			>
				{value}
			</span>
			<button
				type="button"
				className="text-muted-foreground hover:text-foreground rounded p-1 -m-1 transition-colors"
				aria-label={`Copy ${label}`}
			>
				<Icon icon={faCopy} className="w-3" />
			</button>
		</div>
	);
}

function PillTabs({
	tabs,
	active,
	onChange,
}: {
	tabs: string[];
	active: string;
	onChange: (value: string) => void;
}) {
	return (
		<div className="inline-flex items-center gap-0.5 p-0.5 rounded-md border dark:border-white/10 bg-muted/30">
			{tabs.map((tab) => {
				const isActive = tab === active;
				return (
					<button
						key={tab}
						type="button"
						onClick={() => onChange(tab)}
						className={cn(
							"h-6 px-2.5 text-xs rounded-[5px] transition-colors",
							isActive
								? "bg-background text-foreground shadow-sm"
								: "text-muted-foreground hover:text-foreground",
						)}
					>
						{tab}
					</button>
				);
			})}
		</div>
	);
}

function CodeBlock({ lines }: { lines: { tone?: "muted" | "accent" | "default"; text: string }[] }) {
	return (
		<pre className="rounded-md border dark:border-white/10 bg-muted/30 px-3.5 py-3 font-mono text-[11.5px] leading-relaxed overflow-x-auto">
			<code>
				{lines.map((line, i) => (
					<div
						key={i}
						className={cn(
							line.tone === "muted" &&
								"text-muted-foreground",
							line.tone === "accent" && "text-amber-500",
							(line.tone ?? "default") === "default" &&
								"text-foreground",
						)}
					>
						{line.text || "\u00A0"}
					</div>
				))}
			</code>
		</pre>
	);
}

const BACKEND_ENV = [
	{ label: "RIVET_ENDPOINT", value: "https://api.rivet.dev/v1/ns/default" },
	{ label: "RIVET_NAMESPACE", value: "default" },
	{ label: "RIVET_TOKEN", value: "sk_live_••••••••••••••••••••••••••••" },
];

const CLIENT_CODE: Record<string, { tone?: "muted" | "accent" | "default"; text: string }[]> = {
	JavaScript: [
		{ tone: "muted", text: "// client.ts" },
		{ tone: "accent", text: "import" },
		{ text: "{ createClient } " },
		{ tone: "accent", text: "from " },
		{ text: '"rivetkit/client";' },
		{ text: "" },
		{ tone: "accent", text: "export const " },
		{ text: "client = createClient({" },
		{ text: '  endpoint: process.env.RIVET_ENDPOINT,' },
		{ text: "});" },
	],
	React: [
		{ tone: "muted", text: "// App.tsx" },
		{ tone: "accent", text: "import " },
		{ text: "{ RivetProvider } " },
		{ tone: "accent", text: "from " },
		{ text: '"@rivetkit/react";' },
		{ text: "" },
		{ tone: "accent", text: "export default function " },
		{ text: "App() {" },
		{ text: "  return <RivetProvider client={client}>{children}</RivetProvider>" },
		{ text: "}" },
	],
	"Next.js": [
		{ tone: "muted", text: "// app/providers.tsx" },
		{ tone: "accent", text: "'use client'" },
		{ text: "" },
		{ tone: "accent", text: "import " },
		{ text: "{ RivetProvider } " },
		{ tone: "accent", text: "from " },
		{ text: '"@rivetkit/react";' },
		{ text: "" },
		{ tone: "accent", text: "export function " },
		{ text: "Providers({ children }) {" },
		{ text: "  return <RivetProvider client={client}>{children}</RivetProvider>" },
		{ text: "}" },
	],
};

function BackendConfigSection() {
	const [tab, setTab] = useState("JavaScript");
	return (
		<section className="space-y-4">
			<SectionHeading
				title="Backend Configuration"
				description="Use these values to connect your backend to Rivet. Route HTTP requests from your servers and runners."
			/>
			<div className="rounded-lg border dark:border-white/10 overflow-hidden">
				<div className="px-3 py-2 text-[10px] font-medium uppercase tracking-wider text-muted-foreground bg-muted/20 border-b dark:border-white/10 flex items-center justify-between">
					<span>Environment variables</span>
					<button
						type="button"
						className="text-muted-foreground hover:text-foreground transition-colors normal-case tracking-normal text-xs font-normal"
					>
						Copy all
					</button>
				</div>
				{BACKEND_ENV.map((env) => (
					<CopyFieldRow
						key={env.label}
						label={env.label}
						value={env.value}
					/>
				))}
			</div>
			<div className="flex items-center justify-between">
				<PillTabs
					tabs={["JavaScript", "React", "Next.js"]}
					active={tab}
					onChange={setTab}
				/>
				<button
					type="button"
					className="text-xs text-muted-foreground hover:text-foreground transition-colors"
				>
					View docs →
				</button>
			</div>
			<CodeBlock lines={CLIENT_CODE[tab] ?? CLIENT_CODE.JavaScript} />
		</section>
	);
}

const DATACENTERS = [
	{ name: "Northern Virginia", code: "us-east-1", status: "operational" },
	{ name: "Frankfurt", code: "eu-central-1", status: "operational" },
	{ name: "Singapore", code: "ap-southeast-1", status: "operational" },
	{ name: "Oregon", code: "us-west-2", status: "operational" },
];

function DatacenterStatusSection() {
	return (
		<section className="space-y-3">
			<SectionHeading
				title="Datacenter status"
				description="Datacenters where your actors can run. All systems operational."
			/>
			<div className="rounded-lg border dark:border-white/10 overflow-hidden">
				{DATACENTERS.map((dc, i) => (
					<div
						key={dc.code}
						className={cn(
							"flex items-center gap-3 px-3 py-2 text-xs hover:bg-muted/20 transition-colors",
							i !== DATACENTERS.length - 1 &&
								"border-b dark:border-white/10",
						)}
					>
						<span className="flex items-center justify-center size-4 rounded-full bg-green-500/15 text-green-500 shrink-0">
							<Icon icon={faCheck} className="w-2" />
						</span>
						<span className="text-foreground flex-1 min-w-0 truncate">
							{dc.name}
						</span>
						<span className="text-muted-foreground font-mono">
							{dc.code}
						</span>
					</div>
				))}
			</div>
		</section>
	);
}

function ApiTokensSection() {
	return (
		<section className="space-y-3">
			<SectionHeading
				title="API tokens"
				description="Create API tokens for programmatic access. These can manage namespaces and actors."
				badge="Beta"
				action={
					<Button variant="outline" size="sm" className="gap-1.5 shrink-0">
						<Icon icon={faPlus} className="w-3" />
						Create token
					</Button>
				}
			/>
			<div className="rounded-lg border dark:border-white/10 overflow-hidden">
				<div
					className={cn(
						"grid grid-cols-[2fr_2fr_1.4fr_1.2fr] items-center gap-3 px-3 py-2",
						"text-[10px] font-medium uppercase tracking-wider text-muted-foreground",
						"border-b dark:border-white/10 bg-muted/20",
					)}
				>
					<span>Name</span>
					<span>Token</span>
					<span>Created</span>
					<span>Expires</span>
				</div>
				<div className="px-3 py-8 text-center">
					<p className="text-xs text-muted-foreground">
						No API tokens yet. Create one to get started.
					</p>
				</div>
			</div>
		</section>
	);
}

function DangerZoneSection() {
	return (
		<section className="rounded-lg border border-destructive/30 bg-destructive/5 p-4">
			<div className="flex items-start justify-between gap-4">
				<div className="min-w-0">
					<h4 className="text-sm font-semibold text-destructive">
						Archive namespace
					</h4>
					<p className="mt-1 text-xs text-muted-foreground leading-relaxed">
						Archiving stops all actors and revokes all tokens. This action cannot be undone.
					</p>
				</div>
				<Button variant="destructive" size="sm" className="shrink-0">
					Archive
				</Button>
			</div>
		</section>
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
				<AccordionContent className="px-6 py-6 space-y-8">
					<BackendConfigSection />
					<div className="border-t dark:border-white/10" />
					<ApiTokensSection />
					<div className="border-t dark:border-white/10" />
					<DatacenterStatusSection />
					<div className="border-t dark:border-white/10" />
					<DangerZoneSection />
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
