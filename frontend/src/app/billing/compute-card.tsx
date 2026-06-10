import {
	faMemory,
	faMicrochip,
	faServer,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { cn } from "@/components";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { COMPUTE } from "@/content/billing";

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

/** Full-page compute pricing card, matching the usage cards on the billing page. */
export function ComputeUsageCard() {
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

/** Compact compute pricing rows for the settings billing drawer. */
export function ComputeUsageRows() {
	return (
		<div>
			<div className="px-5 pt-5 pb-4">
				<h3 className="text-sm font-semibold text-foreground">
					Compute
				</h3>
				<p className="mt-0.5 text-xs text-muted-foreground">
					Billed per active second by configured CPU and memory.
				</p>
			</div>
			<div className="border-t border-foreground/10">
				{COMPUTE_RATES.map((rate, idx) => (
					<div
						key={rate.label}
						className={cn(
							"flex items-center justify-between px-5 py-3.5",
							idx < COMPUTE_RATES.length - 1 &&
								"border-b border-foreground/10",
						)}
					>
						<div className="flex items-center gap-3">
							<div className="flex size-7 items-center justify-center rounded-md border border-foreground/10">
								<Icon icon={rate.icon} className="size-3.5" />
							</div>
							<span className="text-sm font-medium text-foreground">
								{rate.label}
							</span>
						</div>
						<div className="text-right">
							<div className="text-sm tabular-nums font-medium text-foreground">
								{formatRate(rate.rate)}
							</div>
							<div className="text-[11px] text-muted-foreground">
								{rate.unit}
							</div>
						</div>
					</div>
				))}
			</div>
			<p className="px-5 pt-3 pb-4 text-[11px] text-muted-foreground leading-relaxed">
				{CAPS_NOTE}
			</p>
		</div>
	);
}
