import {
	faArrowUpRightFromSquare,
	faAws,
	faCloud,
	faGoogleCloud,
	faHetzner,
	faMicrosoft,
	faServer,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { Controller, useForm, useWatch } from "react-hook-form";
import { PlanBadge } from "@/app/billing/billing-plan-badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Table, TableBody, TableCell, TableRow } from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { type CloudId, CLOUDS, type Tier, type TierId, TIERS } from "./catalog";
import { IpAllowlist } from "./ip-allowlist";
import {
	type ClusterSelection,
	clampRunners,
	DEFAULT_SELECTION,
	formatUsd,
	getCloud,
	getSize,
	getTier,
	quote,
} from "./model";
import { RunnersConfig } from "./runners-config";

export const CLOUD_ICONS: Record<CloudId, IconProp> = {
	aws: faAws,
	gcp: faGoogleCloud,
	azure: faMicrosoft,
	hetzner: faHetzner,
	byoc: faCloud,
};

function flag(country: string): string {
	return String.fromCodePoint(
		...[...country.toUpperCase()].map((c) => 0x1f1a5 + c.charCodeAt(0)),
	);
}

const RADIO_CLASS =
	"border-foreground/40 text-foreground data-[state=checked]:border-foreground";

interface CreateClusterPageProps {
	defaultValues?: ClusterSelection;
	onCreate: (selection: ClusterSelection) => void;
	isCreating?: boolean;
}

/**
 * Aiven-style single-page cluster creation: cloud and region first (prices
 * depend on them), then a plan, which is a level (nodes, support) plus
 * the hardware each node runs, then the basics. A sticky summary rail on the
 * right prices it.
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

				<Section title="Cloud">
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
										{c.id === "byoc" ? (
											<PlanBadge plan="byoc" />
										) : null}
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

				<Section title="Plan">
					<Controller
						control={form.control}
						name="tier"
						render={({ field }) => (
							<Tabs
								value={field.value}
								onValueChange={(v) => {
									field.onChange(v);
									form.setValue(
										"runners",
										clampRunners(
											v as TierId,
											selection.runners,
										),
									);
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

					<p className="text-sm text-muted-foreground">
						{tier.highlights.join(" · ")}
					</p>
					<ControlPlaneRow tier={tier} />
				</Section>

				<Section title="Runners">
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

				<Section title="Name">
					<Input
						{...form.register("name", { required: true })}
						className="max-w-sm font-mono"
					/>
				</Section>

				<Section title="Allowed IPs">
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

/** The core engine nodes the plan provisions; not user-sized. */
function ControlPlaneRow({ tier }: { tier: Tier }) {
	const size = getSize(tier.controlPlane.size);
	return (
		<div className="flex items-center justify-between gap-4 rounded-lg border border-border px-4 py-3 text-sm">
			<span className="flex items-center gap-3">
				<Icon icon={faServer} className="text-muted-foreground" />
				<span className="flex flex-col">
					<span className="font-medium">Control plane</span>
					<span className="text-xs text-muted-foreground">
						Preconfigured by the {tier.label} plan
					</span>
				</span>
			</span>
			<span className="font-mono text-muted-foreground">
				{tier.controlPlane.nodes} × {size.label}
			</span>
		</div>
	);
}

function SummaryCard({
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
						<PlanBadge plan={q.tier.id} />
						{q.cloud.id === "byoc" ? (
							<PlanBadge plan="byoc" />
						) : null}
					</span>
					<span className="text-muted-foreground">
						{q.controlPlane.nodes} ×{" "}
						<span className="font-mono">
							{q.controlPlane.size.label}
						</span>{" "}
						control plane
					</span>
				</SummaryRow>
				<SummaryRow label="Runners">
					<span>
						{q.runners.count} ×{" "}
						<span className="font-mono">
							{q.runners.size.label}
						</span>
					</span>
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
				{q.cloud.id === "byoc" ? (
					<p className="text-xs text-muted-foreground">
						Platform fee only. Compute is billed by your cloud.
					</p>
				) : null}
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

function SummaryRow({
	label,
	children,
}: {
	label: string;
	children: ReactNode;
}) {
	return (
		<div className="flex flex-col gap-0.5 border-t border-border py-3 text-sm">
			<div className="text-xs text-muted-foreground">{label}</div>
			{children}
		</div>
	);
}

function Section({
	title,
	description,
	children,
}: {
	title: string;
	description?: string;
	children: ReactNode;
}) {
	return (
		<section className="flex flex-col gap-4">
			<div>
				<h2 className="text-lg font-semibold">{title}</h2>
				{description ? (
					<p className="mt-1 text-sm text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			{children}
		</section>
	);
}
