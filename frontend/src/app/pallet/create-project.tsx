import type { Rivet } from "@rivet-gg/cloud";
import {
	faArrowUpRightFromSquare,
	faAws,
	faCircleCheck,
	faGlobe,
	faGoogleCloud,
	faMicrosoft,
	faPlus,
	faServer,
	faShieldHalved,
	faTrash,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { useForm, useFormContext, useWatch } from "react-hook-form";
import { type ApiFieldMap, showApiErrorOnForm } from "@/lib/form-errors";
import z from "zod";
import { PlanBadge } from "@/app/billing/billing-plan-badge";
import { ByocContactTrigger } from "@/app/byoc/byoc-contact-trigger";
import { byocDetailsSchema } from "@/app/forms/create-project-form";
import { LogoMark } from "@/app/logo";
import {
	Badge,
	Button,
	Card,
	cn,
	createSchemaForm,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	Label,
	RadioGroup,
	RadioGroupItem,
	Table,
	TableBody,
	TableCell,
	TableFooter,
	TableHead,
	TableHeader,
	TableRow,
	ToggleGroup,
	ToggleGroupItem,
	toast,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import {
	PALLET_ALLOWANCES,
	PALLET_CLOUDS,
	PALLET_NODE_SIZES,
	PALLET_PLANS,
	PLAN_LABELS,
} from "@/content/billing";

const CLOUD_ICONS = { aws: faAws, gcp: faGoogleCloud, azure: faMicrosoft };

const ids = <T extends { id: string }>(items: readonly T[]) =>
	items.map((item) => item.id) as [T["id"], ...T["id"][]];

const ALLOW_ALL = "0.0.0.0/0";

const cidrSchema = z.string().cidr({
	version: "v4",
	message: "Enter an IPv4 address or CIDR block, e.g. 10.20.0.0/16",
});

const formSchema = z.object({
	plan: z.enum(ids(PALLET_PLANS)),
	cloud: z.enum(ids(PALLET_CLOUDS)),
	region: z.string().nonempty(),
	node_size: z.enum(ids(PALLET_NODE_SIZES)),
	cidr_allowlist: z
		.array(cidrSchema)
		.min(1, "Add at least one IP address range"),
	name: byocDetailsSchema.shape.name,
});

type FormValues = z.infer<typeof formSchema>;

// Path params of `clusters.upsert` and `regions.upsert`, plus the region body.
const API_FIELDS: ApiFieldMap<
	FormValues,
	"cluster" | "region" | keyof Rivet.v2.RegionsUpsertRequest
> = {
	cluster: "name",
	node_count: "plan",
};

const { Form, Submit } = createSchemaForm(formSchema);

const formatUsd = (value: number) => `$${value.toLocaleString("en-US")}`;

const formatVcpu = (value: number) => {
	const denominator = [1, 2, 4, 8].find((d) => Number.isInteger(value * d));
	return denominator && denominator > 1
		? `${value * denominator}/${denominator}`
		: `${value}`;
};

const flag = (country: string) =>
	String.fromCodePoint(...[...country].map((c) => 0x1f1a5 + c.charCodeAt(0)));

function quote(values: Partial<FormValues>) {
	const plan =
		PALLET_PLANS.find((p) => p.id === values.plan) ?? PALLET_PLANS[0];
	const cloud =
		PALLET_CLOUDS.find((c) => c.id === values.cloud) ?? PALLET_CLOUDS[0];
	const size =
		PALLET_NODE_SIZES.find((s) => s.id === values.node_size) ??
		PALLET_NODE_SIZES[0];
	const isFree = plan.monthlyUsd === 0;
	const nodesUsd = (s: (typeof PALLET_NODE_SIZES)[number]) =>
		isFree
			? 0
			: Math.round(s.monthlyUsdPerNode * plan.nodes * cloud.multiplier);
	const storageGb = isFree ? size.storageGb / 2 : size.storageGb;
	const objectStorageGb =
		storageGb * PALLET_ALLOWANCES.objectStorageMultiplier;
	return {
		plan,
		cloud,
		size,
		isFree,
		nodesUsd,
		storageGb,
		objectStorageGb,
		lines: [
			{
				label: `${PLAN_LABELS[plan.id]} plan`,
				amount: isFree ? "Free" : plan.monthlyUsd,
			},
			{
				label: `${plan.nodes} node ${size.label} cluster`,
				amount: isFree ? "Included" : nodesUsd(size),
			},
			{ label: `${storageGb} GB actor storage`, amount: "Included" },
			{
				label: `${objectStorageGb} GB object storage`,
				amount: "Included",
			},
		],
		totalUsd: plan.monthlyUsd + nodesUsd(size),
	};
}

export function CreateProject({ organization }: { organization: string }) {
	const navigate = useNavigate();
	const provider = useCloudDataProvider();
	const queryClient = useQueryClient();
	const { mutateAsync, isPending } = useMutation({
		...provider.createPalletClusterMutationOptions(),
		meta: { hideErrorToast: true },
		onSuccess: async (data) => {
			toast.success("Cluster created");
			await queryClient.invalidateQueries(
				provider.currentOrgPalletClustersQueryOptions(),
			);
			return navigate({
				to: "/orgs/$organization/clusters/$cluster",
				params: { organization, cluster: data.cluster },
			});
		},
	});

	return (
		<Form
			defaultValues={{
				plan: "team",
				cloud: "aws",
				region: PALLET_CLOUDS[0].regions[0].id,
				node_size: "small",
				cidr_allowlist: [ALLOW_ALL],
				name: "",
			}}
			onSubmit={async ({ plan, ...values }, form) => {
				try {
					await mutateAsync({
						...values,
						node_count: quote({ plan }).plan.nodes,
					});
				} catch (error) {
					showApiErrorOnForm(form, error, API_FIELDS);
				}
			}}
		>
			<div className="mx-auto grid w-full max-w-7xl gap-10 px-6 py-10 lg:grid-cols-[minmax(0,1fr)_380px]">
				<div className="flex min-w-0 flex-col gap-12">
					<header>
						<h1 className="text-3xl font-semibold tracking-tight">
							Create a Rivet cluster
						</h1>
						<p className="mt-2 text-muted-foreground">
							A cluster is a dedicated control plane for your
							actors. Pick a plan, a cloud, and a node size. You
							can change all of it later without downtime.
						</p>
					</header>
					<PlanField />
					<CloudField />
					<SizeField />
					<NetworkAccessField />
					<FormField
						name="name"
						render={({ field }) => (
							<Section
								title="Cluster name"
								description="Lowercase letters, numbers, and hyphens. Used in your endpoint hostname and in the dashboard."
							>
								<FormControl>
									<Input
										className="max-w-sm font-mono"
										autoComplete="off"
										placeholder="rivet-prod"
										{...field}
									/>
								</FormControl>
							</Section>
						)}
					/>
				</div>
				<aside className="lg:sticky lg:top-6 lg:self-start">
					<CheckoutCard>
						<Submit
							allowPristine
							size="lg"
							isLoading={isPending}
							className="w-full"
							startIcon={<Icon icon={faServer} />}
						>
							Create cluster
						</Submit>
					</CheckoutCard>
				</aside>
			</div>
		</Form>
	);
}

function Section({
	title,
	description,
	children,
}: {
	title: string;
	description: string;
	children: ReactNode;
}) {
	return (
		<FormItem className="flex flex-col gap-4 space-y-0">
			<div>
				<FormLabel className="text-xl font-semibold">{title}</FormLabel>
				<FormDescription className="mt-1">
					{description}
				</FormDescription>
			</div>
			{children}
			<FormMessage />
		</FormItem>
	);
}

const radioClassName =
	"border-foreground/40 text-foreground data-[state=checked]:border-foreground";

const optionClassName =
	"cursor-pointer border bg-card transition-colors hover:border-muted-foreground/40 has-[[data-state=checked]]:border-foreground has-[[data-state=checked]]:ring-1 has-[[data-state=checked]]:ring-foreground";

function PlanField() {
	return (
		<FormField
			name="plan"
			render={({ field }) => (
				<Section
					title="Plan"
					description="Plans set how many nodes the cluster runs, its uptime SLA, and support."
				>
					<FormControl>
						<RadioGroup
							value={field.value}
							onValueChange={field.onChange}
							className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4"
						>
							{PALLET_PLANS.map((plan) => (
								<Label
									key={plan.id}
									htmlFor={`plan-${plan.id}`}
									className={cn(
										optionClassName,
										"flex flex-col gap-3 rounded-xl p-4 font-normal",
									)}
								>
									<div className="flex items-center justify-between">
										<PlanBadge
											plan={plan.id}
											className="text-sm"
										/>
										<RadioGroupItem
											id={`plan-${plan.id}`}
											value={plan.id}
											className={radioClassName}
										/>
									</div>
									<div className="text-2xl font-semibold">
										{formatUsd(plan.monthlyUsd)}
										<span className="text-sm font-normal text-muted-foreground">
											{" "}
											/mo
										</span>
									</div>
									<p className="text-xs text-muted-foreground">
										{plan.description}
									</p>
									<ul className="mt-auto flex flex-col gap-1 text-xs">
										{plan.highlights.map((highlight) => (
											<li
												key={highlight}
												className="flex items-center gap-2"
											>
												<CheckIcon />
												{highlight}
											</li>
										))}
									</ul>
								</Label>
							))}
						</RadioGroup>
					</FormControl>
				</Section>
			)}
		/>
	);
}

function CloudField() {
	const { setValue } = useFormContext<FormValues>();
	const cloudId = useWatch<FormValues, "cloud">({ name: "cloud" });
	const cloud =
		PALLET_CLOUDS.find((c) => c.id === cloudId) ?? PALLET_CLOUDS[0];
	return (
		<FormField
			name="cloud"
			render={({ field }) => (
				<Section
					title="Cloud"
					description="Where the control plane runs. Node prices below depend on it. Workers can run anywhere and connect to it."
				>
					<FormControl>
						<ToggleGroup
							type="single"
							className="flex-wrap justify-start gap-2"
							value={field.value}
							onValueChange={(value) => {
								const next = PALLET_CLOUDS.find(
									(c) => c.id === value,
								);
								if (!next) {
									return;
								}
								field.onChange(next.id);
								setValue("region", next.regions[0].id);
							}}
						>
							{PALLET_CLOUDS.map((c) => (
								<ToggleGroupItem
									key={c.id}
									value={c.id}
									className="h-11 gap-2 rounded-lg border px-4 data-[state=on]:border-foreground data-[state=on]:bg-muted/60"
								>
									<Icon icon={CLOUD_ICONS[c.id]} />
									{c.label}
								</ToggleGroupItem>
							))}
						</ToggleGroup>
					</FormControl>
					<p className="text-sm text-muted-foreground">
						{cloud.description} {cloud.regions.length} regions.
					</p>
					<FormField
						name="region"
						render={({ field: region }) => (
							<RadioGroup
								value={region.value}
								onValueChange={region.onChange}
								className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3"
							>
								{cloud.regions.map((r) => (
									<Label
										key={r.id}
										htmlFor={`region-${r.id}`}
										className={cn(
											optionClassName,
											"flex items-center gap-3 rounded-lg p-3 font-normal",
										)}
									>
										<RadioGroupItem
											id={`region-${r.id}`}
											value={r.id}
											className={radioClassName}
										/>
										<span className="text-lg leading-none">
											{flag(r.country)}
										</span>
										<span className="flex min-w-0 flex-col">
											<span className="font-mono">
												{r.id}
											</span>
											<span className="text-xs text-muted-foreground">
												{r.city}
											</span>
										</span>
									</Label>
								))}
							</RadioGroup>
						)}
					/>
				</Section>
			)}
		/>
	);
}

const SIZE_GRID =
	"grid grid-cols-[1.5rem_1fr_6rem_7rem_11rem_7rem] items-center gap-3";

function SizeField() {
	const q = quote(useWatch<FormValues>());
	return (
		<FormField
			name="node_size"
			render={({ field }) => (
				<Section
					title="Cluster size"
					description="Every node in the cluster runs this size. Prices are for the whole cluster on the cloud you picked."
				>
					<FormControl>
						<RadioGroup
							value={field.value}
							onValueChange={field.onChange}
							className="gap-2"
						>
							<div
								className={cn(
									SIZE_GRID,
									"rounded-lg border bg-muted/40 px-4 py-2.5 text-xs font-medium text-muted-foreground",
								)}
							>
								<span />
								<span>Size</span>
								<span className="text-right">vCPU / node</span>
								<span className="text-right">
									Memory / node
								</span>
								<span>Nodes</span>
								<span className="text-right">
									Monthly price
								</span>
							</div>
							{PALLET_NODE_SIZES.map((size) => (
								<Label
									key={size.id}
									htmlFor={`size-${size.id}`}
									className={cn(
										optionClassName,
										SIZE_GRID,
										"rounded-lg px-4 py-4 font-normal",
									)}
								>
									<RadioGroupItem
										id={`size-${size.id}`}
										value={size.id}
										className={radioClassName}
									/>
									<span className="w-fit rounded-md border bg-muted/60 px-2 py-0.5 font-mono">
										{size.label}
									</span>
									<span className="text-right tabular-nums">
										{formatVcpu(size.vcpu)}
									</span>
									<span className="text-right tabular-nums">
										{size.memoryGb} GB
									</span>
									<span className="text-muted-foreground">
										{q.plan.nodes === 1
											? "1 node"
											: `${q.plan.nodes} nodes, high availability`}
									</span>
									<span className="text-right font-semibold tabular-nums text-emerald-600 dark:text-emerald-400">
										{q.isFree
											? "Free"
											: formatUsd(q.nodesUsd(size))}
									</span>
								</Label>
							))}
						</RadioGroup>
					</FormControl>
					<ByocContactTrigger>
						{(open) => (
							<button
								type="button"
								onClick={open}
								className="ml-auto flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
							>
								Request larger cluster sizes
								<Icon icon={faArrowUpRightFromSquare} />
							</button>
						)}
					</ByocContactTrigger>
				</Section>
			)}
		/>
	);
}

function NetworkAccessField() {
	const draft = useForm({ defaultValues: { cidr: "" } });
	return (
		<FormField
			name="cidr_allowlist"
			render={({ field }) => {
				const value: string[] = field.value;
				// Not `handleSubmit`: it would submit the enclosing cluster form.
				const add = () => {
					const input = draft.getValues("cidr").trim();
					const cidr = input.includes("/") ? input : `${input}/32`;
					const parsed = cidrSchema.safeParse(cidr);
					if (!parsed.success) {
						draft.setError("cidr", {
							message: parsed.error.issues[0].message,
						});
						return;
					}
					if (value.includes(cidr)) {
						draft.setError("cidr", {
							message: "Already in the allowlist",
						});
						return;
					}
					field.onChange([...value, cidr]);
					draft.resetField("cidr");
				};
				return (
					<Section
						title="Network access"
						description="Allowed inbound IP addresses. Only these ranges can reach the external endpoint."
					>
						<ul className="flex flex-col divide-y rounded-lg border">
							{value.map((cidr) => (
								<li
									key={cidr}
									className="flex items-center gap-3 px-3 py-2 text-sm"
								>
									<Icon
										icon={
											cidr === ALLOW_ALL
												? faGlobe
												: faShieldHalved
										}
										className={
											cidr === ALLOW_ALL
												? "text-warning"
												: "text-muted-foreground"
										}
									/>
									<span className="flex-1 font-mono">
										{cidr}
									</span>
									{cidr === ALLOW_ALL ? (
										<Badge variant="warning">
											Allows all traffic
										</Badge>
									) : null}
									<Button
										type="button"
										variant="ghost"
										size="icon-sm"
										aria-label={`Remove ${cidr}`}
										onClick={() =>
											field.onChange(
												value.filter((v) => v !== cidr),
											)
										}
									>
										<Icon icon={faTrash} />
									</Button>
								</li>
							))}
							{value.length === 0 ? (
								<li className="flex items-center gap-2 px-3 py-3 text-sm text-destructive">
									<Icon icon={faTriangleExclamation} />
									No addresses allowed. Nothing can connect to
									this cluster until you add a range.
								</li>
							) : null}
						</ul>
						<div className="flex items-start gap-2">
							<div className="flex-1">
								<Input
									{...draft.register("cidr")}
									placeholder="10.20.0.0/16"
									className="font-mono"
									aria-label="IP address or CIDR block"
									aria-invalid={!!draft.formState.errors.cidr}
									onKeyDown={(e) => {
										if (e.key === "Enter") {
											e.preventDefault();
											add();
										}
									}}
								/>
								{draft.formState.errors.cidr ? (
									<p className="mt-1 text-xs text-destructive">
										{draft.formState.errors.cidr.message}
									</p>
								) : null}
							</div>
							<Button
								type="button"
								variant="outline"
								startIcon={<Icon icon={faPlus} />}
								onClick={add}
							>
								Add IP address range
							</Button>
						</div>
						{value.includes(ALLOW_ALL) ? (
							<p className="text-xs text-muted-foreground">
								<span className="text-warning">
									{ALLOW_ALL}
								</span>{" "}
								allows connections from any IP address. Remove
								it and add your application servers and CI
								ranges before going to production.
							</p>
						) : null}
					</Section>
				);
			}}
		/>
	);
}

function CheckoutCard({ children }: { children: ReactNode }) {
	const values = useWatch<FormValues>();
	const q = quote(values);
	const allowlist = values.cidr_allowlist ?? [];
	return (
		<Card className="flex flex-col overflow-hidden">
			<div className="flex flex-col gap-5 p-6">
				<div className="flex items-center gap-3">
					<LogoMark className="size-9 shrink-0" />
					<h2 className="text-lg font-semibold">
						Your new Rivet cluster
					</h2>
				</div>
				<p className="text-sm text-muted-foreground">
					Your {q.size.label} cluster on {q.cloud.label} includes:
				</p>
				<Include
					title={
						q.plan.nodes === 1
							? "1 node"
							: `${q.plan.nodes} nodes, high availability`
					}
					detail={
						q.plan.sla
							? `${q.plan.sla} uptime SLA`
							: "No uptime SLA"
					}
				>
					<Table className="mt-2 text-xs">
						<TableHeader>
							<TableRow className="border-0 hover:bg-transparent">
								<TableHead className="h-6 px-0" />
								<TableHead className="h-6 px-0 text-right">
									CPU
								</TableHead>
								<TableHead className="h-6 px-0 text-right">
									Memory
								</TableHead>
							</TableRow>
						</TableHeader>
						<TableBody className="tabular-nums">
							<TableRow className="border-0 hover:bg-transparent">
								<TableCell className="px-0 py-0.5 text-muted-foreground">
									{q.plan.nodes}× {q.size.label}
								</TableCell>
								<TableCell className="px-0 py-0.5 text-right">
									{formatVcpu(q.size.vcpu)} vCPU
								</TableCell>
								<TableCell className="px-0 py-0.5 text-right">
									{q.size.memoryGb} GB
								</TableCell>
							</TableRow>
						</TableBody>
						<TableFooter className="bg-transparent font-medium">
							<TableRow className="hover:bg-transparent">
								<TableCell className="px-0 pt-1.5">
									Total resources
								</TableCell>
								<TableCell className="px-0 pt-1.5 text-right">
									{formatVcpu(q.size.vcpu * q.plan.nodes)}{" "}
									vCPU
								</TableCell>
								<TableCell className="px-0 pt-1.5 text-right">
									{q.size.memoryGb * q.plan.nodes} GB
								</TableCell>
							</TableRow>
						</TableFooter>
					</Table>
				</Include>
				<Include
					title={`${q.storageGb} GB actor storage`}
					detail={`SQLite per actor. ${formatUsd(PALLET_ALLOWANCES.extraStorageUsdPerGb)}/GB above that.`}
				/>
				<Include
					title={`${q.objectStorageGb} GB object storage`}
					detail={`S3-compatible bucket for blobs and uploads. ${formatUsd(PALLET_ALLOWANCES.extraObjectStorageUsdPerGb)}/GB above that.`}
				/>
				<Include
					title={`${PALLET_ALLOWANCES.includedEgressGb} GB egress bandwidth`}
					detail={`Billed at ${formatUsd(PALLET_ALLOWANCES.egressUsdPerGb)}/GB above ${PALLET_ALLOWANCES.includedEgressGb} GB included.`}
				/>
				<Include
					title="Allowed IPs"
					detail={
						allowlist.length === 0
							? "No inbound access configured"
							: allowlist.join(", ")
					}
				/>
			</div>
			<div className="flex flex-col gap-4 border-t bg-muted/30 p-6">
				<dl className="flex flex-col gap-1.5 text-sm">
					{q.lines.map((line) => (
						<div
							key={line.label}
							className="flex justify-between gap-4"
						>
							<dt className="text-muted-foreground">
								{line.label}
							</dt>
							<dd
								className={cn(
									"tabular-nums",
									typeof line.amount === "number"
										? "text-emerald-600 dark:text-emerald-400"
										: "text-muted-foreground",
								)}
							>
								{typeof line.amount === "number"
									? formatUsd(line.amount)
									: line.amount}
							</dd>
						</div>
					))}
				</dl>
				<div className="flex items-baseline justify-between border-t pt-4">
					<span className="text-lg font-semibold">Monthly cost:</span>
					<span className="text-2xl font-semibold tabular-nums text-emerald-600 dark:text-emerald-400">
						{formatUsd(q.totalUsd)}
					</span>
				</div>
				{children}
				<p className="text-center text-xs text-muted-foreground">
					Usage-based compute for workers is billed separately.
				</p>
			</div>
		</Card>
	);
}

function Include({
	title,
	detail,
	children,
}: {
	title: string;
	detail: string;
	children?: ReactNode;
}) {
	return (
		<div className="flex gap-3 border-t pt-4">
			<CheckIcon className="mt-0.5" />
			<div className="min-w-0 flex-1">
				<div className="text-sm font-semibold">{title}</div>
				<p className="mt-0.5 break-words text-xs text-muted-foreground">
					{detail}
				</p>
				{children}
			</div>
		</div>
	);
}

function CheckIcon({ className }: { className?: string }) {
	return (
		<Icon
			icon={faCircleCheck}
			className={cn(
				"shrink-0 text-emerald-600 dark:text-emerald-400",
				className,
			)}
		/>
	);
}
