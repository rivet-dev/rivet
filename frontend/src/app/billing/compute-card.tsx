import {
	faArrowUpRight,
	faMemory,
	faMicrochip,
	faServer,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { formatCurrency } from "@/components";
import { useHasManagedPool } from "@/components/actors/actor-details-shared";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { COMPUTE } from "@/content/billing";

/**
 * Renders children only when the current namespace actually has Compute enabled
 * (a managed pool). Must be rendered inside a namespace context, since
 * `useHasManagedPool` reads the namespace data provider.
 */
export function IfNamespaceHasCompute({ children }: { children: ReactNode }) {
	return useHasManagedPool() ? <>{children}</> : null;
}

interface ComputeRate {
	icon: IconProp;
	label: string;
	rate: number;
	unit: string;
}

const COMPUTE_RATES: ComputeRate[] = [
	{
		icon: faMicrochip,
		label: "CPU",
		rate: COMPUTE.cpuPerVcpuSecond,
		unit: "per vCPU-second",
	},
	{
		icon: faMemory,
		label: "Memory",
		rate: COMPUTE.memoryPerGibSecond,
		unit: "per GiB-second",
	},
];

/** Renders the per-second compute rate, e.g. `$0.0000330`. */
function formatRate(rate: number): string {
	return `$${rate.toFixed(7)}`;
}

const CAPS_NOTE = `Billed per active second. Up to ${COMPUTE.maxVcpu} vCPU (Free plan is limited to ${COMPUTE.freeMaxVcpu} vCPU and capped at $5/month of compute). One vCPU is half a physical core. You can also bring your own compute and run your actors and applications on AWS, Vercel, Railway, or bare metal, paid directly to your provider.`;

interface ComputeUsageCardProps {
	/**
	 * Month-to-date compute cost in dollars. When omitted (e.g. the
	 * project-scoped billing page before its aggregate endpoint exists), the
	 * card shows pricing only without a usage figure.
	 */
	monthToDate?: number;
	isLoading?: boolean;
	isError?: boolean;
}

/** Full-page compute pricing card, matching the usage cards on the billing page. */
export function ComputeUsageCard({
	monthToDate,
	isLoading,
	isError,
}: ComputeUsageCardProps = {}) {
	return (
		<Card className="w-full border border-border bg-card shadow-sm">
			<CardHeader className="flex flex-row items-start justify-between space-y-0 pb-4">
				<div className="flex items-start gap-3">
					<div className="flex h-8 w-8 items-center justify-center rounded-full border border-border">
						<Icon
							icon={faServer}
							className="h-4 w-4 text-foreground"
						/>
					</div>
					<div>
						<h3 className="text-base font-semibold text-foreground">
							Compute
						</h3>
						<p className="text-sm text-muted-foreground">
							Run your actors and applications on Rivet Compute,
							billed per active second by configured CPU and
							memory.
						</p>
					</div>
				</div>
				{monthToDate !== undefined && (
					<div className="text-right ml-6">
						<div className="text-2xl font-semibold text-foreground">
							{isLoading ? (
								<Skeleton className="h-8 w-20" />
							) : isError ? (
								"—"
							) : (
								formatCurrency(monthToDate)
							)}
						</div>
						<div className="text-sm text-muted-foreground">
							Month-to-date
						</div>
					</div>
				)}
			</CardHeader>
			<CardContent className="pt-0">
				<div className="rounded-lg border border-border bg-muted/30 divide-y divide-border">
					{COMPUTE_RATES.map((rate) => (
						<div
							key={rate.label}
							className="flex items-center justify-between px-5 py-3.5"
						>
							<div className="flex items-center gap-3">
								<Icon
									icon={rate.icon}
									className="size-4 text-muted-foreground"
								/>
								<span className="text-sm font-medium text-foreground">
									{rate.label}
								</span>
							</div>
							<div className="text-right">
								<span className="text-sm tabular-nums font-medium text-foreground">
									{formatRate(rate.rate)}
								</span>
								<span className="ml-2 text-xs text-muted-foreground">
									{rate.unit}
								</span>
							</div>
						</div>
					))}
				</div>
				<p className="mt-3 text-xs text-muted-foreground leading-relaxed">
					{CAPS_NOTE}
				</p>
			</CardContent>
		</Card>
	);
}

/**
 * Pointer shown on namespace surfaces. Compute is billed per project, so the
 * namespace billing/metrics views direct users to Project Billing rather than
 * showing a (namespace-less) compute figure. The link opens the project billing
 * drawer on the current route.
 */
export function ComputeUsageProjectBillingPointer() {
	return (
		<Card className="w-full border border-border bg-card shadow-sm">
			<CardHeader className="flex flex-row items-start justify-between space-y-0 pb-4">
				<div className="flex items-start gap-3">
					<div className="flex h-8 w-8 items-center justify-center rounded-full border border-border">
						<Icon
							icon={faServer}
							className="h-4 w-4 text-foreground"
						/>
					</div>
					<div>
						<h3 className="text-base font-semibold text-foreground">
							Compute
						</h3>
						<p className="text-sm text-muted-foreground">
							Compute is billed per project, not per namespace.
						</p>
					</div>
				</div>
			</CardHeader>
			<CardContent className="pt-0">
				<Link
					to="."
					search={(prev) => ({ ...prev, settings: "billing" })}
					className="inline-flex items-center gap-1.5 text-sm font-medium text-primary hover:underline"
				>
					See Compute Usage on Project Billing
					<Icon icon={faArrowUpRight} className="size-3" />
				</Link>
			</CardContent>
		</Card>
	);
}

