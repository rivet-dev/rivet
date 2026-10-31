import {
	faArrowLeft,
	faArrowRight,
	faBolt,
	faCheck,
	faChevronDown,
	faChevronRight,
	faCloud,
	faCube,
	faHardDrive,
	faLayerGroup,
	faMemory,
	faMicrochip,
	faPlus,
	faServer,
	faShieldHalved,
	faSliders,
	faTag,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { type ReactNode, useState } from "react";
import {
	Controller,
	type UseFormReturn,
	useForm,
	useWatch,
} from "react-hook-form";
import { PlanBadge } from "@/app/billing/billing-plan-badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Table, TableBody, TableCell, TableRow } from "@/components/ui/table";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/components/lib/utils";
import {
	type Cloud,
	type CloudId,
	CLOUDS,
	formatVcpu,
	type Tier,
	type TierId,
	TIERS,
} from "./catalog";
import { Highlight } from "./highlight";
import {
	CLOUD_ICONS,
	flag,
	PlanOptions,
	RADIO_CLASS,
	Section,
	Spec,
	SpecLine,
	SummaryCard,
} from "./create-cluster-page";
import { IpAllowlist } from "./ip-allowlist";
import {
	type ClusterSelection,
	DEFAULT_SELECTION,
	formatDiskGb,
	formatMemory,
	formatThroughput,
	formatUsd,
	getCloud,
	getPlanOption,
	getSize,
	getTier,
	nodeMonthlyUsd,
	planMonthlyUsd,
	planWriteMbps,
	quote,
	selectTier,
} from "./model";
import { RunnersConfig } from "./runners-config";

// Explorations of a simplified create flow: pick a name, a region and a plan
// level (Hobby / Startup / Business) and everything else is configured from
// the plan's defaults. Each variant is reachable at
// /mock/cluster-onboarding?create=<variant>.

export type CreateVariant = "fold" | "card" | "wizard" | "sentence" | "dialog";

export const CREATE_VARIANTS: { id: CreateVariant; label: string }[] = [
	{ id: "fold", label: "A · Fold-down advanced settings" },
	{ id: "card", label: "B · Centered summary card" },
	{ id: "wizard", label: "C · Three-step wizard" },
	{ id: "sentence", label: "D · One sentence" },
	{ id: "dialog", label: "E · Dialog over cluster list" },
];

interface VariantProps {
	defaultValues?: ClusterSelection;
	onCreate: (selection: ClusterSelection) => void;
	isCreating?: boolean;
}

export function CreateClusterVariant({
	variant,
	...props
}: VariantProps & { variant: CreateVariant }) {
	switch (variant) {
		case "fold":
			return <FoldDownVariant {...props} />;
		case "card":
			return <CenteredCardVariant {...props} />;
		case "wizard":
			return <WizardVariant {...props} />;
		case "sentence":
			return <SentenceVariant {...props} />;
		case "dialog":
			return <DialogVariant {...props} />;
	}
}

// ---------------------------------------------------------------------------
// Shared form plumbing

type ClusterForm = UseFormReturn<ClusterSelection>;

function useClusterForm(defaultValues: ClusterSelection) {
	const form = useForm<ClusterSelection>({ defaultValues });
	const selection = useWatch({ control: form.control }) as ClusterSelection;
	const setTier = (tier: TierId) => {
		const next = selectTier(selection, tier);
		form.setValue("tier", next.tier);
		form.setValue("plan", next.plan);
		form.setValue("runners", next.runners);
	};
	const setCloud = (cloud: CloudId) => {
		form.setValue("cloud", cloud);
		form.setValue("region", getCloud(cloud).regions[0].id);
	};
	return {
		form,
		selection,
		cloud: getCloud(selection.cloud),
		tier: getTier(selection.tier),
		setTier,
		setCloud,
	};
}

/** The configuration a plan level starts with: its default control plane option and runners. */
function tierDefaults(tier: Tier, cloud: Cloud) {
	const plan = getPlanOption(tier, tier.defaultPlan);
	const cp = getSize(plan.size);
	const runners = getSize(tier.defaultRunners.size);
	const usd =
		planMonthlyUsd(plan, cloud) +
		tier.defaultRunners.count * nodeMonthlyUsd(runners, cloud);
	return { plan, cp, runners, usd };
}

// ---------------------------------------------------------------------------
// Shared controls

function CloudPills({
	value,
	onChange,
	size = "default",
}: {
	value: CloudId;
	onChange: (cloud: CloudId) => void;
	size?: "default" | "sm";
}) {
	return (
		<ToggleGroup
			type="single"
			value={value}
			onValueChange={(v) => {
				if (v) onChange(v as CloudId);
			}}
			className="flex-wrap justify-start gap-2"
		>
			{CLOUDS.map((c) => (
				<ToggleGroupItem
					key={c.id}
					value={c.id}
					className={cn(
						"gap-2 rounded-lg border border-border data-[state=on]:border-foreground data-[state=on]:bg-muted/60",
						size === "sm" ? "h-9 px-3" : "h-11 px-4",
					)}
				>
					<Icon icon={CLOUD_ICONS[c.id]} />
					{c.label}
				</ToggleGroupItem>
			))}
		</ToggleGroup>
	);
}

function RegionSelect({
	cloud,
	value,
	onChange,
	className,
}: {
	cloud: Cloud;
	value: string;
	onChange: (region: string) => void;
	className?: string;
}) {
	return (
		<Select value={value} onValueChange={onChange}>
			<SelectTrigger className={cn("h-11", className)}>
				<SelectValue />
			</SelectTrigger>
			<SelectContent>
				{cloud.regions.map((r) => (
					<SelectItem key={r.id} value={r.id}>
						<span className="flex items-center gap-2">
							<span className="text-base leading-none">
								{flag(r.country)}
							</span>
							<span className="font-mono">{r.id}</span>
							<span className="text-muted-foreground">
								{r.city}
							</span>
						</span>
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}

/** Region radio table from the full create page, for the variants that keep it. */
function RegionTable({
	cloud,
	value,
	onChange,
}: {
	cloud: Cloud;
	value: string;
	onChange: (region: string) => void;
}) {
	return (
		<RadioGroup value={value} onValueChange={onChange} className="block">
			<Table containerClassName="rounded-lg border border-border">
				<TableBody>
					{cloud.regions.map((region) => (
						<TableRow
							key={region.id}
							data-state={
								value === region.id ? "selected" : undefined
							}
							className="cursor-pointer"
						>
							<TableCell className="w-10 py-2.5 pr-0">
								<RadioGroupItem
									id={`region-${region.id}`}
									value={region.id}
									className={RADIO_CLASS}
								/>
							</TableCell>
							<TableCell className="w-56 py-2.5">
								<Label
									htmlFor={`region-${region.id}`}
									className="cursor-pointer font-mono"
								>
									{region.id}
								</Label>
							</TableCell>
							<TableCell className="py-2.5">
								<Label
									htmlFor={`region-${region.id}`}
									className="flex cursor-pointer items-center gap-2 text-muted-foreground"
								>
									<span className="text-base leading-none">
										{flag(region.country)}
									</span>
									{region.city}
								</Label>
							</TableCell>
						</TableRow>
					))}
				</TableBody>
			</Table>
		</RadioGroup>
	);
}

/**
 * The three plan levels as selectable cards. Each shows what its default
 * configuration costs on the chosen cloud, since that is what gets created.
 */
function TierCards({
	cloud,
	value,
	onChange,
	compact,
}: {
	cloud: Cloud;
	value: TierId;
	onChange: (tier: TierId) => void;
	compact?: boolean;
}) {
	return (
		<div
			className={cn(
				"grid gap-3",
				compact ? "grid-cols-3" : "sm:grid-cols-3",
			)}
		>
			{TIERS.map((t) => {
				const d = tierDefaults(t, cloud);
				const selected = value === t.id;
				return (
					<button
						key={t.id}
						type="button"
						aria-pressed={selected}
						onClick={() => onChange(t.id)}
						className={cn(
							"flex flex-col gap-1 rounded-lg border text-left transition-colors",
							compact ? "p-3" : "gap-3 p-4",
							selected
								? "border-foreground bg-muted/60"
								: "border-border hover:border-foreground/40",
						)}
					>
						<div className="flex items-start justify-between gap-2">
							<span className="font-semibold">{t.label}</span>
							<span
								className={cn(
									"flex size-4 shrink-0 items-center justify-center rounded-full border",
									selected
										? "border-foreground bg-foreground text-background"
										: "border-foreground/40",
								)}
							>
								{selected ? (
									<Icon icon={faCheck} className="size-2.5" />
								) : null}
							</span>
						</div>
						<div className="text-lg font-semibold tabular-nums">
							{formatUsd(d.usd)}
							<span className="text-sm font-normal text-muted-foreground">
								/mo
							</span>
						</div>
						{compact ? null : (
							<>
								<p className="text-sm text-muted-foreground">
									{t.description}
								</p>
								<ul className="flex flex-col gap-1 text-sm text-muted-foreground">
									{t.highlights.map((h) => (
										<li
											key={h}
											className="flex items-center gap-2"
										>
											<Icon
												icon={faCheck}
												className="size-3 text-emerald-500"
											/>
											<Highlight text={h} />
										</li>
									))}
								</ul>
							</>
						)}
					</button>
				);
			})}
		</div>
	);
}

/** What the plan configured on the user's behalf, as read-only lines. */
function AutoConfigured({ selection }: { selection: ClusterSelection }) {
	const q = quote(selection);
	const rows = [
		{
			key: "plan",
			label: "Control Plane",
			value: <PlanSummary selection={selection} />,
		},
		{
			key: "runners",
			label: "Actor Nodes",
			value: (
				<span className="font-mono">
					{q.runners.count} × {formatVcpu(q.runners.size.vcpu)} vCPU /{" "}
					{formatMemory(q.runners.size.memoryMb)}
				</span>
			),
		},
		{
			key: "allowlist",
			label: "Allowed IPs",
			value: (
				<span className="font-mono">
					{selection.allowlist.length === 0
						? "None"
						: selection.allowlist.join(", ")}
				</span>
			),
		},
	];
	return (
		<dl className="divide-y divide-border rounded-lg border border-border text-sm">
			{rows.map((r) => (
				<div
					key={r.key}
					className="flex items-center gap-3 px-4 py-2.5"
				>
					<dt className="w-32 shrink-0 text-muted-foreground">
						{r.label}
					</dt>
					<dd className="min-w-0 flex-1 truncate">{r.value}</dd>
				</div>
			))}
		</dl>
	);
}

function PlanSummary({ selection }: { selection: ClusterSelection }) {
	const q = quote(selection);
	return (
		<span className="font-mono">
			{q.plan.label}
			<span className="text-muted-foreground">
				{" "}
				· {q.controlPlane.nodes} ×{" "}
				{formatVcpu(q.controlPlane.size.vcpu)} vCPU /{" "}
				{formatMemory(q.controlPlane.size.memoryMb)}
			</span>
		</span>
	);
}

// ---------------------------------------------------------------------------
// The "Configuration" block in the dialog: a summary list where each row
// drills into its full editor in place, with a back button to return.

type ConfigKey = "plan" | "runners" | "allowlist";

interface ConfiguredProps {
	form: ClusterForm;
	tier: Tier;
	cloud: Cloud;
	selection: ClusterSelection;
}

function Configuration({ form, tier, cloud, selection }: ConfiguredProps) {
	const [open, setOpen] = useState<ConfigKey | null>(null);
	const q = quote(selection);
	const rows: {
		key: ConfigKey;
		label: string;
		/** Shown under the label in the left column. */
		badge?: ReactNode;
		value: ReactNode;
		editor: ReactNode;
	}[] = [
		{
			key: "plan",
			label: "Control Plane",
			badge: <PlanBadge plan={q.tier.id} label={q.plan.label} />,
			value: (
				<div className="flex flex-col gap-1.5">
					<SpecLine>
						<Spec icon={faLayerGroup}>
							{q.controlPlane.nodes}{" "}
							{q.controlPlane.nodes === 1 ? "server" : "servers"}
						</Spec>
						<Spec icon={faMicrochip}>
							{formatVcpu(q.controlPlane.size.vcpu)} vCPU
						</Spec>
						<Spec icon={faMemory}>
							{formatMemory(q.controlPlane.size.memoryMb)} RAM
						</Spec>
						<Spec icon={faBolt}>
							{formatDiskGb(
								q.controlPlane.size.diskCacheGb *
									q.controlPlane.nodes,
							)}{" "}
							hot cache
						</Spec>
						<Spec icon={faHardDrive}>
							{formatThroughput(planWriteMbps(q.plan))} writes
						</Spec>
					</SpecLine>
				</div>
			),
			editor: (
				<Controller
					control={form.control}
					name="plan"
					render={({ field }) => (
						<PlanOptions
							key={tier.id}
							tier={tier}
							cloud={cloud}
							value={field.value}
							onChange={field.onChange}
						/>
					)}
				/>
			),
		},
		{
			key: "runners",
			label: "Actor Nodes",
			value: (
				<SpecLine>
					<Spec icon={faLayerGroup}>
						{q.runners.count}{" "}
						{q.runners.count === 1 ? "server" : "servers"}
					</Spec>
					<Spec icon={faMicrochip}>
						{formatVcpu(q.runners.size.vcpu)} vCPU
					</Spec>
					<Spec icon={faMemory}>
						{formatMemory(q.runners.size.memoryMb)} RAM
					</Spec>
				</SpecLine>
			),
			editor: (
				<Controller
					control={form.control}
					name="runners"
					render={({ field }) => (
						<RunnersConfig
							tier={tier}
							cloud={cloud}
							value={field.value}
							onChange={field.onChange}
						/>
					)}
				/>
			),
		},
		{
			key: "allowlist",
			label: "Allowed IPs",
			value: (
				<span className="font-mono">
					{selection.allowlist.length === 0
						? "None"
						: selection.allowlist.join(", ")}
				</span>
			),
			editor: (
				<Controller
					control={form.control}
					name="allowlist"
					render={({ field }) => (
						<IpAllowlist
							value={field.value}
							onChange={field.onChange}
						/>
					)}
				/>
			),
		},
	];
	const active = rows.find((r) => r.key === open);

	if (active) {
		return (
			<div
				key={active.key}
				className="flex min-w-0 flex-col gap-3 animate-in fade-in-0 slide-in-from-right-2 duration-200 [&_td]:whitespace-nowrap [&_td]:px-2 [&_th]:px-2 [&_th]:whitespace-nowrap"
			>
				<div className="flex items-center gap-2">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						onClick={() => setOpen(null)}
						startIcon={<Icon icon={faArrowLeft} />}
						className="-ml-2 h-7 px-2 text-muted-foreground"
					>
						Back
					</Button>
					<span className="text-sm font-medium">{active.label}</span>
				</div>
				{active.editor}
			</div>
		);
	}

	return (
		<div className="flex flex-col gap-2 animate-in fade-in-0 slide-in-from-left-2 duration-200">
			<div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
				Configuration
			</div>
			<div className="divide-y divide-border rounded-lg border border-border text-sm">
				{rows.map((r) => (
					<button
						key={r.key}
						type="button"
						onClick={() => setOpen(r.key)}
						className="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors first:rounded-t-lg last:rounded-b-lg hover:bg-foreground/[0.04]"
					>
						<span className="flex w-32 shrink-0 flex-col items-start gap-1.5 self-start text-muted-foreground">
							{r.label}
							{r.badge}
						</span>
						<span className="min-w-0 flex-1">{r.value}</span>
						<Icon
							icon={faChevronRight}
							className="size-3 shrink-0 text-muted-foreground"
						/>
					</button>
				))}
			</div>
		</div>
	);
}

/** The full-page controls behind "Advanced settings": plan size, runners, allowlist. */
function AdvancedSettings({
	form,
	tier,
	cloud,
}: {
	form: ClusterForm;
	tier: Tier;
	cloud: Cloud;
}) {
	return (
		<div className="flex flex-col gap-8">
			<Field title="Control Plane size">
				<Controller
					control={form.control}
					name="plan"
					render={({ field }) => (
						<PlanOptions
							key={tier.id}
							tier={tier}
							cloud={cloud}
							value={field.value}
							onChange={field.onChange}
						/>
					)}
				/>
			</Field>
			<Field
				title="Actor Nodes"
				description="Connected to the control plane and run your actor logic. Scale them with the number of actors running."
			>
				<Controller
					control={form.control}
					name="runners"
					render={({ field }) => (
						<RunnersConfig
							tier={tier}
							cloud={cloud}
							value={field.value}
							onChange={field.onChange}
						/>
					)}
				/>
			</Field>
			<Field title="Allowed IPs">
				<Controller
					control={form.control}
					name="allowlist"
					render={({ field }) => (
						<IpAllowlist
							value={field.value}
							onChange={field.onChange}
						/>
					)}
				/>
			</Field>
		</div>
	);
}

function Field({
	title,
	description,
	children,
}: {
	title: string;
	description?: string;
	children: ReactNode;
}) {
	return (
		<div className="flex flex-col gap-3">
			<div>
				<h3 className="text-sm font-semibold">{title}</h3>
				{description ? (
					<p className="mt-0.5 text-sm text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			{children}
		</div>
	);
}

function PriceFooter({
	selection,
	isCreating,
	className,
}: {
	selection: ClusterSelection;
	isCreating?: boolean;
	className?: string;
}) {
	const q = quote(selection);
	return (
		<div
			className={cn("flex items-center justify-between gap-4", className)}
		>
			<div className="flex items-baseline gap-2">
				<span className="text-2xl font-semibold tabular-nums">
					{formatUsd(q.totalUsd)}
				</span>
				<span className="text-sm text-muted-foreground">per month</span>
			</div>
			<Button
				type="submit"
				size="lg"
				isLoading={isCreating}
				startIcon={<Icon icon={faServer} />}
			>
				Create cluster
			</Button>
		</div>
	);
}

// ---------------------------------------------------------------------------
// A. Current layout, plan levels as cards, advanced settings folded away.

function FoldDownVariant({
	defaultValues = DEFAULT_SELECTION,
	onCreate,
	isCreating,
}: VariantProps) {
	const { form, selection, cloud, tier, setTier, setCloud } =
		useClusterForm(defaultValues);
	const [advanced, setAdvanced] = useState(false);

	return (
		<form
			onSubmit={form.handleSubmit(onCreate)}
			className="mx-auto grid w-full max-w-7xl gap-10 px-6 py-10 lg:grid-cols-[minmax(0,1fr)_360px]"
		>
			<div className="flex min-w-0 flex-col gap-10">
				<header>
					<h1 className="text-2xl font-semibold tracking-tight">
						Create a cluster
					</h1>
				</header>

				<Section icon={faTag} title="Name">
					<Input
						{...form.register("name", { required: true })}
						className="max-w-sm font-mono"
					/>
				</Section>

				<Section icon={faCloud} title="Cloud">
					<CloudPills value={selection.cloud} onChange={setCloud} />
					<Controller
						control={form.control}
						name="region"
						render={({ field }) => (
							<RegionTable
								cloud={cloud}
								value={field.value}
								onChange={field.onChange}
							/>
						)}
					/>
				</Section>

				<Section icon={faCube} title="Plan">
					<TierCards
						cloud={cloud}
						value={selection.tier}
						onChange={setTier}
					/>
				</Section>

				<Section icon={faSliders} title="Advanced settings">
					{advanced ? null : <AutoConfigured selection={selection} />}
					<Button
						type="button"
						variant="outline"
						className="w-fit"
						onClick={() => setAdvanced((v) => !v)}
						endIcon={
							<Icon
								icon={faChevronDown}
								className={cn(
									"transition-transform",
									advanced && "rotate-180",
								)}
							/>
						}
					>
						{advanced
							? "Hide advanced settings"
							: "Customize control plane, actor nodes and allowed IPs"}
					</Button>
					{advanced ? (
						<div className="pt-2">
							<AdvancedSettings
								form={form}
								tier={tier}
								cloud={cloud}
							/>
						</div>
					) : null}
				</Section>
			</div>

			<aside className="lg:sticky lg:top-6 lg:self-start">
				<SummaryCard selection={selection} isCreating={isCreating} />
			</aside>
		</form>
	);
}

// ---------------------------------------------------------------------------
// B. The summary rail is the whole page: one centered card you edit inline.

function CenteredCardVariant(props: VariantProps) {
	return (
		<div className="mx-auto w-full max-w-xl px-6 py-14">
			<Card className="overflow-hidden">
				<CompactForm {...props} title="Create a cluster" />
			</Card>
		</div>
	);
}

/**
 * The compact editable summary used by B and E. Name, cloud and plan are
 * edited in place; the rest is a configuration list that drills into each
 * value's full editor.
 */
function CompactForm({
	defaultValues = DEFAULT_SELECTION,
	onCreate,
	isCreating,
	title,
}: VariantProps & { title?: string }) {
	const { form, selection, cloud, tier, setTier, setCloud } =
		useClusterForm(defaultValues);
	return (
		<form
			onSubmit={form.handleSubmit(onCreate)}
			className="flex min-w-0 flex-col"
		>
			{title ? (
				<h1 className="px-6 pt-6 text-xl font-semibold tracking-tight">
					{title}
				</h1>
			) : null}
			<div className="flex flex-col divide-y divide-border px-6">
				<CardRow label="Name">
					<Input
						{...form.register("name", { required: true })}
						className="font-mono"
					/>
				</CardRow>
				<CardRow label="Cloud">
					<div className="flex flex-col gap-2">
						<CloudPills
							value={selection.cloud}
							onChange={setCloud}
							size="sm"
						/>
						<Controller
							control={form.control}
							name="region"
							render={({ field }) => (
								<RegionSelect
									cloud={cloud}
									value={field.value}
									onChange={field.onChange}
									className="h-10"
								/>
							)}
						/>
					</div>
				</CardRow>
				<CardRow label="Plan">
					<TierCards
						cloud={cloud}
						value={selection.tier}
						onChange={setTier}
						compact
					/>
					<ul className="mt-3 flex flex-col gap-1 text-sm text-muted-foreground">
						{tier.highlights.map((h) => (
							<li key={h} className="flex items-center gap-2">
								<Icon
									icon={faCheck}
									className="size-3 text-emerald-500"
								/>
								<Highlight text={h} />
							</li>
						))}
					</ul>
				</CardRow>
				<div className="py-5">
					<Configuration
						form={form}
						tier={tier}
						cloud={cloud}
						selection={selection}
					/>
				</div>
			</div>
			<PriceFooter
				selection={selection}
				isCreating={isCreating}
				className="border-t border-border bg-muted/30 px-6 py-4"
			/>
		</form>
	);
}

function CardRow({ label, children }: { label: string; children: ReactNode }) {
	return (
		<div className="flex flex-col gap-2 py-5">
			<div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
				{label}
			</div>
			{children}
		</div>
	);
}

// ---------------------------------------------------------------------------
// C. One decision per screen.

const WIZARD_STEPS = ["Basics", "Plan", "Review"] as const;

function WizardVariant({
	defaultValues = DEFAULT_SELECTION,
	onCreate,
	isCreating,
}: VariantProps) {
	const { form, selection, cloud, setTier, setCloud } =
		useClusterForm(defaultValues);
	const [step, setStep] = useState(0);

	return (
		<form
			onSubmit={form.handleSubmit(onCreate)}
			className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-6 py-14"
		>
			<header className="flex flex-col gap-6">
				<h1 className="text-2xl font-semibold tracking-tight">
					Create a cluster
				</h1>
				<ol className="flex items-center gap-3 text-sm">
					{WIZARD_STEPS.map((s, i) => (
						<li key={s} className="flex items-center gap-3">
							<button
								type="button"
								onClick={() => i < step && setStep(i)}
								className={cn(
									"flex items-center gap-2",
									i === step
										? "text-foreground"
										: "text-muted-foreground",
								)}
							>
								<span
									className={cn(
										"flex size-6 items-center justify-center rounded-full border text-xs tabular-nums",
										i < step
											? "border-foreground bg-foreground text-background"
											: i === step
												? "border-foreground"
												: "border-border",
									)}
								>
									{i < step ? (
										<Icon
											icon={faCheck}
											className="size-3"
										/>
									) : (
										i + 1
									)}
								</span>
								{s}
							</button>
							{i < WIZARD_STEPS.length - 1 ? (
								<span className="h-px w-8 bg-border" />
							) : null}
						</li>
					))}
				</ol>
			</header>

			{step === 0 ? (
				<div className="flex flex-col gap-6">
					<Field title="Name">
						<Input
							{...form.register("name", { required: true })}
							className="max-w-sm font-mono"
						/>
					</Field>
					<Field title="Cloud">
						<CloudPills
							value={selection.cloud}
							onChange={setCloud}
						/>
					</Field>
					<Field title="Region">
						<Controller
							control={form.control}
							name="region"
							render={({ field }) => (
								<RegionTable
									cloud={cloud}
									value={field.value}
									onChange={field.onChange}
								/>
							)}
						/>
					</Field>
				</div>
			) : null}

			{step === 1 ? (
				<TierCards
					cloud={cloud}
					value={selection.tier}
					onChange={setTier}
				/>
			) : null}

			{step === 2 ? (
				<SummaryCard selection={selection} isCreating={isCreating} />
			) : null}

			{step < 2 ? (
				<div className="flex items-center justify-between">
					<Button
						type="button"
						variant="ghost"
						disabled={step === 0}
						onClick={() => setStep((s) => s - 1)}
						startIcon={<Icon icon={faArrowLeft} />}
					>
						Back
					</Button>
					<Button
						type="button"
						onClick={() => setStep((s) => s + 1)}
						endIcon={<Icon icon={faArrowRight} />}
					>
						Continue
					</Button>
				</div>
			) : (
				<Button
					type="button"
					variant="ghost"
					className="w-fit"
					onClick={() => setStep(1)}
					startIcon={<Icon icon={faArrowLeft} />}
				>
					Back
				</Button>
			)}
		</form>
	);
}

// ---------------------------------------------------------------------------
// D. The whole request as one sentence with inline controls.

function SentenceVariant({
	defaultValues = DEFAULT_SELECTION,
	onCreate,
	isCreating,
}: VariantProps) {
	const { form, selection, cloud, tier, setTier, setCloud } =
		useClusterForm(defaultValues);
	const [advanced, setAdvanced] = useState(false);
	const q = quote(selection);

	return (
		<form
			onSubmit={form.handleSubmit(onCreate)}
			className="mx-auto flex w-full max-w-4xl flex-col gap-10 px-6 py-16"
		>
			<p className="flex flex-wrap items-center gap-x-2 gap-y-3 text-2xl font-medium leading-relaxed tracking-tight">
				<span>Create</span>
				<Input
					{...form.register("name", { required: true })}
					className="h-11 w-56 border-0 border-b border-foreground/40 bg-transparent px-1 font-mono text-2xl shadow-none focus-visible:ring-0"
				/>
				<span>on</span>
				<Select
					value={selection.cloud}
					onValueChange={(v) => setCloud(v as CloudId)}
				>
					<SelectTrigger className="h-11 w-auto gap-2 border-0 border-b border-foreground/40 bg-transparent px-1 text-2xl shadow-none focus:ring-0">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{CLOUDS.map((c) => (
							<SelectItem key={c.id} value={c.id}>
								<span className="flex items-center gap-2">
									<Icon icon={CLOUD_ICONS[c.id]} />
									{c.label}
								</span>
							</SelectItem>
						))}
					</SelectContent>
				</Select>
				<span>in</span>
				<Controller
					control={form.control}
					name="region"
					render={({ field }) => (
						<Select
							value={field.value}
							onValueChange={field.onChange}
						>
							<SelectTrigger className="h-11 w-auto gap-2 border-0 border-b border-foreground/40 bg-transparent px-1 font-mono text-2xl shadow-none focus:ring-0">
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{cloud.regions.map((r) => (
									<SelectItem key={r.id} value={r.id}>
										<span className="flex items-center gap-2">
											<span>{flag(r.country)}</span>
											<span className="font-mono">
												{r.id}
											</span>
											<span className="text-muted-foreground">
												{r.city}
											</span>
										</span>
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					)}
				/>
				<span>on the</span>
				<Select
					value={selection.tier}
					onValueChange={(v) => setTier(v as TierId)}
				>
					<SelectTrigger className="h-11 w-auto gap-2 border-0 border-b border-foreground/40 bg-transparent px-1 text-2xl shadow-none focus:ring-0">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{TIERS.map((t) => (
							<SelectItem key={t.id} value={t.id}>
								{t.label}
								<span className="ml-2 text-muted-foreground">
									from {formatUsd(tierDefaults(t, cloud).usd)}
									/mo
								</span>
							</SelectItem>
						))}
					</SelectContent>
				</Select>
				<span>plan.</span>
			</p>

			<div className="grid gap-3 sm:grid-cols-3">
				<Tile
					icon={faLayerGroup}
					title="Control Plane"
					lines={[
						q.plan.label,
						`${q.controlPlane.nodes} × ${formatVcpu(q.controlPlane.size.vcpu)} vCPU / ${formatMemory(q.controlPlane.size.memoryMb)}`,
						q.controlPlane.nodes >= 3
							? "3 availability zones"
							: "1 availability zone",
					]}
				/>
				<Tile
					icon={faServer}
					title="Actor Nodes"
					lines={[
						`${q.runners.count} ${q.runners.count === 1 ? "server" : "servers"}`,
						`${formatVcpu(q.runners.size.vcpu)} vCPU / ${formatMemory(q.runners.size.memoryMb)} each`,
						tier.highlights[1],
					]}
				/>
				<Tile
					icon={faShieldHalved}
					title="Network"
					lines={[
						`Allowed IPs: ${selection.allowlist.join(", ") || "none"}`,
						`https://api.${selection.name}.rivet.run`,
						tier.highlights[2],
					]}
				/>
			</div>

			<div className="flex flex-col gap-6">
				<PriceFooter selection={selection} isCreating={isCreating} />
				<Collapsible open={advanced} onOpenChange={setAdvanced}>
					<CollapsibleTrigger asChild>
						<button
							type="button"
							className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
						>
							<Icon icon={faSliders} />
							Advanced settings
							<Icon
								icon={faChevronDown}
								className={cn(
									"size-3 transition-transform",
									advanced && "rotate-180",
								)}
							/>
						</button>
					</CollapsibleTrigger>
					<CollapsibleContent className="pt-6">
						<AdvancedSettings
							form={form}
							tier={tier}
							cloud={cloud}
						/>
					</CollapsibleContent>
				</Collapsible>
			</div>
		</form>
	);
}

function Tile({
	icon,
	title,
	lines,
}: {
	icon: IconProp;
	title: string;
	lines: string[];
}) {
	return (
		<div className="flex flex-col gap-2 rounded-lg border border-border p-4">
			<div className="flex items-center gap-2 text-sm font-medium">
				<Icon icon={icon} className="size-3.5 text-muted-foreground" />
				{title}
			</div>
			<ul className="flex flex-col gap-0.5 text-sm text-muted-foreground">
				{lines.map((l) => (
					<li key={l} className="truncate">
						{l}
					</li>
				))}
			</ul>
		</div>
	);
}

// ---------------------------------------------------------------------------
// E. The compact form in a dialog over the clusters list, like PlanetScale.

function DialogVariant(props: VariantProps) {
	const [open, setOpen] = useState(true);
	return (
		<div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-6 py-10">
			<header className="flex items-center justify-between">
				<h1 className="text-2xl font-semibold tracking-tight">
					Clusters
				</h1>
				<Button
					type="button"
					onClick={() => setOpen(true)}
					startIcon={<Icon icon={faPlus} />}
				>
					New cluster
				</Button>
			</header>
			<div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border py-24 text-center">
				<Icon
					icon={faServer}
					className="size-6 text-muted-foreground"
				/>
				<p className="font-medium">No clusters yet</p>
				<p className="text-sm text-muted-foreground">
					Create one to get an endpoint and an admin token.
				</p>
			</div>
			<Dialog open={open} onOpenChange={setOpen}>
				<DialogContent className="max-w-2xl gap-0 p-0 outline-none">
					<DialogHeader className="px-6 pt-6">
						<DialogTitle>New cluster</DialogTitle>
					</DialogHeader>
					<CompactForm {...props} />
				</DialogContent>
			</Dialog>
		</div>
	);
}
