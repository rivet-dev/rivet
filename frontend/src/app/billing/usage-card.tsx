"use client";

import { faInfoCircle, Icon, type IconProp } from "@rivet-gg/icons";
import {
	animate,
	domAnimation,
	LazyMotion,
	motion,
	useMotionValue,
	useTransform,
} from "framer-motion";
import { useEffect, useState } from "react";
import { cn, formatCurrency, WithTooltip } from "@/components";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@/components/ui/collapsible";

export type MetricType = "hours" | "bytes" | "operations";

function stripTrailingZeros(value: number, decimals: number): string {
	return value.toFixed(decimals).replace(/\.?0+$/, "");
}

function formatMetricValue(value: bigint, type: MetricType): string {
	const num = Number(value);

	switch (type) {
		case "hours": {
			const hours = num / 3600;
			if (hours >= 1000) {
				return `${stripTrailingZeros(hours / 1000, 1)}k hrs`;
			}
			return `${stripTrailingZeros(hours, 1)} hrs`;
		}
		case "bytes": {
			const KB = 1000;
			const MB = 1000 * 1000;
			const GB = 1000 * 1000 * 1000;
			const TB = 1000 * 1000 * 1000 * 1000;

			if (num >= TB) {
				return `${stripTrailingZeros(num / TB, 2)} TB`;
			}
			if (num >= GB) {
				return `${stripTrailingZeros(num / GB, 2)} GB`;
			}
			if (num >= MB) {
				return `${stripTrailingZeros(num / MB, 2)} MB`;
			}
			if (num >= KB) {
				return `${stripTrailingZeros(num / KB, 2)} KB`;
			}
			return `${num} B`;
		}
		case "operations": {
			const units = num / 4096;
			if (units >= 1_000_000_000) {
				return `${stripTrailingZeros(units / 1_000_000_000, 2)}B ops`;
			}
			if (units >= 1_000_000) {
				return `${stripTrailingZeros(units / 1_000_000, 2)}M ops`;
			}
			if (units >= 1_000) {
				return `${stripTrailingZeros(units / 1_000, 2)}K ops`;
			}
			return `${Math.round(units)} ops`;
		}
	}
}

const calculatePercent = (value: bigint, max: bigint): number => {
	if (max === 0n) return 0;
	return Number((value * 10000n) / max) / 100;
};

interface AnimatedMetricProps {
	value: bigint;
	type: MetricType;
}

function AnimatedMetric({ value, type }: AnimatedMetricProps) {
	const numValue = Number(value);
	const motionValue = useMotionValue(0);
	const displayValue = useTransform(motionValue, (v) =>
		formatMetricValue(BigInt(Math.round(v)), type),
	);

	useEffect(() => {
		animate(motionValue, numValue, { duration: 1, ease: "circIn" });
	}, [motionValue, numValue]);

	return (
		<LazyMotion features={domAnimation}>
			<motion.span>{displayValue}</motion.span>
		</LazyMotion>
	);
}

interface UsageItem {
	name: string;
	monthToDate: bigint;
}

interface BillingCardProps {
	title: string;
	description: string;
	current: bigint;
	includedInPlan?: bigint;
	monthToDate: bigint;
	metricType: MetricType;
	items?: UsageItem[];
	icon: IconProp;
	footerLabel?: string;
}

export function UsageCard({
	title,
	description,
	current,
	includedInPlan,
	monthToDate,
	metricType,
	items = [],
	icon,
	footerLabel,
}: BillingCardProps) {
	const [isOpen, setIsOpen] = useState(false);

	const maxValue = includedInPlan
		? current > includedInPlan
			? current
			: (includedInPlan * 100n) / 75n
		: current || 1n;

	const currentPercent = calculatePercent(current, maxValue);
	const includedPercent = includedInPlan
		? calculatePercent(includedInPlan, maxValue)
		: 0;

	return (
		<Card className="w-full border border-border bg-card shadow-sm">
			<CardHeader className="flex flex-row items-start justify-between space-y-0 pb-4">
				<div className="flex items-start gap-3">
					<div className="flex h-8 w-8 items-center justify-center rounded-full border border-border">
						<Icon
							icon={icon}
							className={`h-4 w-4 text-foreground`}
						/>
					</div>
					<div>
						<h3 className="text-base font-semibold text-foreground">
							{title}
						</h3>
						<p className="text-sm text-muted-foreground">
							{description}
						</p>
					</div>
				</div>
			</CardHeader>
			<CardContent className="space-y-0 pt-0">
				<div className="rounded-lg border border-border bg-muted/30 p-5">
					<Collapsible
						disabled
						open={isOpen}
						onOpenChange={setIsOpen}
					>
						<CollapsibleTrigger className="flex w-full items-center justify-between text-left">
							<div className="flex items-start gap-3 flex-1">
								<div className="flex-1">
									<div
										className={`relative pb-[calc(60px-6px)] ${includedInPlan ? "pt-[calc(60px-6px)]" : "pt-2"}`}
									>
										{includedInPlan && (
											<div
												className={cn(
													"absolute top-0  border-muted-foreground/60 pb-4 z-[1]",
													includedPercent > 50
														? "border-r-2 pr-2 text-right"
														: "border-l-2 pl-2 text-left",
												)}
												style={{
													left: `${includedPercent}%`,
													transform:
														includedPercent > 50
															? "translateX(-100%)"
															: "translateX(0)",
												}}
											>
												<span className="text-sm text-muted-foreground">
													Included in Plan
												</span>
												<div className="text-sm font-medium text-foreground">
													{formatMetricValue(
														includedInPlan,
														metricType,
													)}
												</div>
											</div>
										)}

										<div className="relative h-1.5 w-full rounded-full bg-border">
											<motion.div
												className="absolute h-1.5 rounded-l-full bg-primary"
												initial={{ width: 0 }}
												animate={{
													width: `${currentPercent}%`,
												}}
												transition={{
													duration: 1,
													ease: "circIn",
												}}
											/>
										</div>

										<div
											className={cn(
												"absolute bottom-0 border-muted-foreground/60 pt-4",
												currentPercent > 50
													? "border-r-2 pr-2 text-right"
													: "border-l-2 pl-2 text-left",
											)}
											style={{
												left: `${currentPercent}%`,
												transform:
													currentPercent > 50
														? "translateX(-100%)"
														: "translateX(0)",
											}}
										>
											<span className="text-sm text-muted-foreground">
												Current
											</span>
											<div className="text-sm font-medium text-foreground">
												<AnimatedMetric
													value={current}
													type={metricType}
												/>
											</div>
										</div>
									</div>
								</div>
							</div>

							<div className="flex items-start text-right ml-6">
								<div>
									<div className="text-2xl font-semibold text-foreground">
										{formatCurrency(monthToDate)}
									</div>
									<div className="flex items-center justify-end gap-1 text-sm text-muted-foreground">
										<span>Month-to-date</span>
										<WithTooltip
											delayDuration={0}
											trigger={
												<Button
													variant="ghost"
													size="icon"
													className="size-auto p-0"
												>
													<Icon icon={faInfoCircle} />
												</Button>
											}
											content="Total charges for the current billing period"
										/>
									</div>
								</div>
							</div>
						</CollapsibleTrigger>

						<CollapsibleContent>
							{items.length > 0 && (
								<div className="mt-4 border-t border-border pt-3 ml-7">
									{items.map((item) => (
										<div
											key={item.name}
											className="flex w-full items-center justify-between py-3"
										>
											<span className="text-sm text-muted-foreground">
												{item.name}
											</span>
											<div className="text-right">
												<div className="text-sm font-semibold text-foreground">
													{formatCurrency(
														item.monthToDate,
													)}
												</div>
												<div className="text-xs text-muted-foreground">
													Month-to-date
												</div>
											</div>
										</div>
									))}
								</div>
							)}
						</CollapsibleContent>
					</Collapsible>
				</div>

				{footerLabel && (
					<div className="border-t border-border px-1 pt-4 mt-4">
						<div className="text-sm font-medium text-muted-foreground">
							{footerLabel}
						</div>
					</div>
				)}
			</CardContent>
		</Card>
	);
}
