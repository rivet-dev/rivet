import {
	faArrowUpRightFromSquare,
	faAws,
	faCheck,
	faChevronDown,
	faChevronUp,
	faBolt,
	faCloud,
	faCube,
	faGoogleCloud,
	faHardDrive,
	faInfoCircle,
	faLayerGroup,
	faMemory,
	faMicrochip,
	faServer,
	faShieldHalved,
	faTag,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { type ReactNode, useState } from "react";
import { Controller, useForm, useWatch } from "react-hook-form";
import { PlanBadge } from "@/app/billing/billing-plan-badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { WithTooltip } from "@/components/ui/tooltip";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
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
import { IpAllowlist } from "./ip-allowlist";
import {
	type ClusterSelection,
	DEFAULT_SELECTION,
	formatDiskGb,
	formatMemory,
	formatThroughput,
	formatUsd,
	getCloud,
	getSize,
	getTier,
	planMonthlyUsd,
	planWriteMbps,
	quote,
	selectTier,
} from "./model";
import { RunnersConfig } from "./runners-config";

export const CLOUD_ICONS: Record<CloudId, IconProp> = {
	aws: faAws,
	gcp: faGoogleCloud,
};

export function flag(country: string): string {
	return String.fromCodePoint(
		...[...country.toUpperCase()].map((c) => 0x1f1a5 + c.charCodeAt(0)),
	);
}

export const RADIO_CLASS =
	"border-foreground/40 text-foreground data-[state=checked]:border-foreground";

interface CreateClusterPageProps {
	defaultValues?: ClusterSelection;
	onCreate: (selection: ClusterSelection) => void;
	isCreating?: boolean;
}

/**
 * Aiven-style single-page cluster creation: cloud and region first (prices
 * depend on them), then a plan, which is a level (SLA, support, limits) with
 * a table of control plane sizes to buy it in, then runners and the basics.
 * A sticky summary rail on the right prices it.
 */
export function CreateClusterPage({
	defaultValues = DEFAULT_SELECTION,
	onCreate,
	isCreating,
}: CreateClusterPageProps) {
	const form = useForm<ClusterSelection>({ defaultValues });
	const selection = useWatch({ control: form.control }) as ClusterSelection;
	const cloud = getCloud(selection.cloud);
	const tier = getTier(selection.tier);

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
					<Controller
						control={form.control}
						name="cloud"
						render={({ field }) => (
							<ToggleGroup
								type="single"
								value={field.value}
								onValueChange={(v) => {
									if (!v) return;
									field.onChange(v);
									form.setValue(
										"region",
										getCloud(v as CloudId).regions[0].id,
									);
								}}
								className="flex-wrap justify-start gap-2"
							>
								{CLOUDS.map((c) => (
									<ToggleGroupItem
										key={c.id}
										value={c.id}
										className="h-11 gap-2 rounded-lg border border-border px-4 data-[state=on]:border-foreground data-[state=on]:bg-muted/60"
									>
										<Icon icon={CLOUD_ICONS[c.id]} />
										{c.label}
									</ToggleGroupItem>
								))}
							</ToggleGroup>
						)}
					/>
					<Controller
						control={form.control}
						name="region"
						render={({ field }) => (
							<RadioGroup
								value={field.value}
								onValueChange={field.onChange}
								className="block"
							>
								<Table containerClassName="rounded-lg border border-border">
									<TableBody>
										{cloud.regions.map((region) => {
											const selected =
												field.value === region.id;
											return (
												<TableRow
													key={region.id}
													data-state={
														selected
															? "selected"
															: undefined
													}
													className="cursor-pointer"
												>
													<TableCell className="w-10 py-2.5 pr-0">
														<RadioGroupItem
															id={`region-${region.id}`}
															value={region.id}
															className={
																RADIO_CLASS
															}
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
																{flag(
																	region.country,
																)}
															</span>
															{region.city}
														</Label>
													</TableCell>
												</TableRow>
											);
										})}
									</TableBody>
								</Table>
							</RadioGroup>
						)}
					/>
				</Section>

				<Section icon={faCube} title="Plan">
					<Controller
						control={form.control}
						name="tier"
						render={({ field }) => (
							<Tabs
								value={field.value}
								onValueChange={(v) => {
									const next = selectTier(
										selection,
										v as TierId,
									);
									field.onChange(next.tier);
									form.setValue("plan", next.plan);
									form.setValue("runners", next.runners);
								}}
							>
								<TabsList className="justify-start">
									{TIERS.map((t) => (
										<TabsTrigger
											key={t.id}
											value={t.id}
											className="[&>span]:bg-foreground"
										>
											{t.label}
										</TabsTrigger>
									))}
								</TabsList>
							</Tabs>
						)}
					/>
					<TierHighlights tier={tier} />
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
				</Section>

				<Section
					icon={faServer}
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
					<a
						href="https://rivet.dev/sales"
						target="_blank"
						rel="noreferrer"
						className="ml-auto flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
					>
						Need a larger size?
						<Icon icon={faArrowUpRightFromSquare} />
					</a>
				</Section>

				<Section icon={faShieldHalved} title="Allowed IPs">
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
				</Section>
			</div>

			<aside className="lg:sticky lg:top-6 lg:self-start">
				<SummaryCard selection={selection} isCreating={isCreating} />
			</aside>
		</form>
	);
}

/** What the level buys regardless of size: node range, limits, support. */
export function TierHighlights({ tier }: { tier: Tier }) {
	return (
		<div className="flex flex-col gap-2 rounded-lg border border-border p-4">
			<p className="text-sm font-medium">{tier.description}</p>
			<ul className="flex flex-col gap-1 text-sm text-muted-foreground">
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
		</div>
	);
}

const COLLAPSED_OPTIONS = 4;

/** The control plane configurations a plan is sold in, priced for the chosen cloud. */
export function PlanOptions({
	tier,
	cloud,
	value,
	onChange,
}: {
	tier: Tier;
	cloud: Cloud;
	value: string;
	onChange: (plan: string) => void;
}) {
	const [expanded, setExpanded] = useState(false);
	const collapsible = tier.options.length > COLLAPSED_OPTIONS;
	const options =
		collapsible && !expanded
			? tier.options.slice(0, COLLAPSED_OPTIONS)
			: tier.options;
	return (
		<div className="flex flex-col gap-3">
			<RadioGroup
				value={value}
				onValueChange={onChange}
				className="block"
			>
				<Table containerClassName="rounded-lg border border-border">
					<TableHeader>
						<TableRow className="hover:bg-transparent">
							<TableHead className="w-10 pr-0" />
							<TableHead>Plan</TableHead>
							<TableHead className="whitespace-nowrap text-right">
								Nodes
							</TableHead>
							<TableHead className="whitespace-nowrap text-right">
								vCPU
							</TableHead>
							<TableHead className="whitespace-nowrap text-right">
								RAM
							</TableHead>
							<TableHead className="whitespace-nowrap text-right">
								<HeadWithInfo info="Data kept in the cluster for fast reads. Anything not in the hot cache is read from cold storage.">
									Hot cache
								</HeadWithInfo>
							</TableHead>
							<TableHead className="whitespace-nowrap text-right">
								<HeadWithInfo info="How much data can be written per second across the whole cluster.">
									Write throughput
								</HeadWithInfo>
							</TableHead>
							<TableHead className="whitespace-nowrap text-right">
								Monthly price
							</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody className="tabular-nums">
						{options.map((option) => {
							const size = getSize(option.size);
							const selected = value === option.id;
							const usd = planMonthlyUsd(option, cloud);
							return (
								<TableRow
									key={option.id}
									data-state={
										selected ? "selected" : undefined
									}
									className="cursor-pointer"
								>
									<TableCell className="py-2.5 pr-0">
										<RadioGroupItem
											id={`plan-${option.id}`}
											value={option.id}
											className={RADIO_CLASS}
										/>
									</TableCell>
									<TableCell className="py-2.5">
										<Label
											htmlFor={`plan-${option.id}`}
											className="cursor-pointer whitespace-nowrap font-mono"
										>
											{option.label}
										</Label>
									</TableCell>
									<TableCell className="py-2.5 text-right text-muted-foreground">
										{option.nodes}
									</TableCell>
									<TableCell className="py-2.5 text-right text-muted-foreground">
										{formatVcpu(size.vcpu)}
									</TableCell>
									<TableCell className="py-2.5 text-right text-muted-foreground">
										{formatMemory(size.memoryMb)}
									</TableCell>
									<TableCell className="py-2.5 text-right text-muted-foreground">
										{formatDiskGb(
											size.diskCacheGb * option.nodes,
										)}
									</TableCell>
									<TableCell className="py-2.5 text-right text-muted-foreground">
										{formatThroughput(
											planWriteMbps(option),
										)}
									</TableCell>
									<TableCell className="py-2.5 text-right font-medium">
										{usd === 0 ? "Free" : formatUsd(usd)}
									</TableCell>
								</TableRow>
							);
						})}
					</TableBody>
				</Table>
			</RadioGroup>
			{collapsible ? (
				<button
					type="button"
					onClick={() => setExpanded((e) => !e)}
					className="flex w-fit items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
				>
					<Icon icon={expanded ? faChevronUp : faChevronDown} />
					{expanded
						? "Show fewer"
						: `View all ${tier.options.length} sizes`}
				</button>
			) : null}
		</div>
	);
}

/** Column header with a hover hint, right-aligned like the numeric columns. */
function HeadWithInfo({
	info,
	children,
}: {
	info: string;
	children: ReactNode;
}) {
	return (
		<span className="inline-flex items-center justify-end gap-1.5">
			{children}
			<WithTooltip
				delayDuration={0}
				trigger={
					<Icon
						icon={faInfoCircle}
						className="size-3 text-muted-foreground/60"
					/>
				}
				content={info}
				contentProps={{ className: "max-w-64" }}
			/>
		</span>
	);
}

export function SummaryCard({
	selection,
	isCreating,
}: {
	selection: ClusterSelection;
	isCreating?: boolean;
}) {
	const q = quote(selection);
	const region = q.cloud.regions.find((r) => r.id === selection.region);
	return (
		<Card className="flex flex-col overflow-hidden">
			<div className="flex flex-col px-6 pt-6">
				<h2 className="pb-3 text-base font-semibold">Summary</h2>
				<SummaryRow label="Name">
					<span className="font-mono">{selection.name || "—"}</span>
				</SummaryRow>
				<SummaryRow label="Cloud">
					<span className="flex items-center gap-2">
						<Icon icon={CLOUD_ICONS[q.cloud.id]} />
						{q.cloud.label}
						{region ? (
							<span className="font-mono text-muted-foreground">
								{region.id}
							</span>
						) : null}
					</span>
				</SummaryRow>
				<SummaryRow label="Plan">
					<span className="flex items-center gap-2">
						<PlanBadge plan={q.tier.id} label={q.plan.label} />
					</span>
					<SpecLine>
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
					</SpecLine>
					<SpecLine>
						<Spec icon={faHardDrive}>
							{formatThroughput(planWriteMbps(q.plan))} writes
						</Spec>
					</SpecLine>
					<SpecLine>
						<Spec icon={faLayerGroup}>
							{q.controlPlane.nodes}{" "}
							{q.controlPlane.nodes === 1 ? "server" : "servers"}{" "}
							across{" "}
							{q.controlPlane.nodes >= 3
								? "3 availability zones"
								: "1 availability zone"}
						</Spec>
					</SpecLine>
				</SummaryRow>
				<SummaryRow label="Actor Nodes">
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
				</SummaryRow>
				<SummaryRow label="Allowed IPs">
					<span className="break-words font-mono text-muted-foreground">
						{selection.allowlist.length === 0
							? "None"
							: selection.allowlist.join(", ")}
					</span>
				</SummaryRow>
			</div>

			<div className="flex flex-col gap-4 p-6">
				<div className="flex items-baseline justify-between">
					<span className="text-sm text-muted-foreground">
						Per month
					</span>
					<span className="text-2xl font-semibold tabular-nums">
						{formatUsd(q.totalUsd)}
					</span>
				</div>
				<Button
					type="submit"
					size="lg"
					className="w-full"
					isLoading={isCreating}
					startIcon={<Icon icon={faServer} />}
				>
					Create cluster
				</Button>
			</div>
		</Card>
	);
}

/** One line of icon-prefixed specs in the summary, wrapping when narrow. */
export function SpecLine({ children }: { children: ReactNode }) {
	return (
		<div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
			{children}
		</div>
	);
}

export function Spec({
	icon,
	children,
}: {
	icon: IconProp;
	children: ReactNode;
}) {
	return (
		<span className="inline-flex items-center gap-1.5 whitespace-nowrap tabular-nums">
			<Icon icon={icon} className="size-3.5 text-muted-foreground" />
			{children}
		</span>
	);
}

export function SummaryRow({
	label,
	children,
}: {
	label: string;
	children: ReactNode;
}) {
	return (
		<div className="flex flex-col gap-1.5 border-t border-border py-3 text-sm">
			<div className="text-xs text-muted-foreground">{label}</div>
			{children}
		</div>
	);
}

export function Section({
	icon,
	title,
	description,
	children,
}: {
	icon: IconProp;
	title: string;
	description?: string;
	children: ReactNode;
}) {
	// Each section renders its own slice of the stepper gutter. The line runs
	// from below the icon through the parent's gap so it joins the next section.
	return (
		<section className="group grid grid-cols-[2rem_minmax(0,1fr)] gap-x-4">
			<div className="relative flex justify-center">
				<span className="flex size-8 shrink-0 items-center justify-center text-muted-foreground">
					<Icon icon={icon} className="size-5" />
				</span>
				<span
					aria-hidden
					className="absolute -bottom-8 left-1/2 top-10 w-px -translate-x-1/2 bg-border group-last-of-type:hidden"
				/>
			</div>
			<div className="flex min-w-0 flex-col gap-4">
				<div className="flex min-h-8 flex-col justify-center">
					<h2 className="text-lg font-semibold leading-tight">
						{title}
					</h2>
					{description ? (
						<p className="mt-1 text-sm text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{children}
			</div>
		</section>
	);
}
