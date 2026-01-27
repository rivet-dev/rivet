"use client";

import {
	faEllipsis,
	faFileLines,
	faInfoCircle,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { useState } from "react";
import { cn } from "@/components";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { WithTooltip } from "@/components/ui/tooltip";

interface UsageItem {
	name: string;
	monthToDate: number;
}

interface BillingCardProps {
	title: string;
	description: string;
	current: number;
	includedInPlan?: number;
	monthToDate: number;
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
	items = [],
	icon,
	footerLabel,
}: BillingCardProps) {
	const [isOpen, setIsOpen] = useState(false);

	const formatCurrency = (value: number) => {
		return `$${value.toFixed(2)}`;
	};

	// Calculate the maximum value for the progress bar scale
	// - If current > includedInPlan: current is 100%
	// - Otherwise: includedInPlan is at 75%, with buffer to 100%
	const maxValue = includedInPlan
		? current > includedInPlan
			? current
			: includedInPlan / 0.75
		: current || 1;

	// Position percentages for the progress bar
	const currentPercent = (current / maxValue) * 100;
	const includedPercent = includedInPlan
		? (includedInPlan / maxValue) * 100
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
				<div className="flex items-center gap-1">
					<Button
						variant="ghost"
						size="icon"
						startIcon={
							<Icon icon={faFileLines} className="h-4 w-4" />
						}
					>
						<span className="sr-only">View documentation</span>
					</Button>
					<Button
						variant="ghost"
						size="icon"
						startIcon={<Icon icon={faEllipsis} />}
					>
						<span className="sr-only">More options</span>
					</Button>
				</div>
			</CardHeader>
			<CardContent className="space-y-0 pt-0">
				{/* Usage Overview Section */}
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
													{includedInPlan}
												</div>
											</div>
										)}

										<div className="relative h-1.5 w-full rounded-full bg-border">
											<div
												className="absolute h-1.5 rounded-l-full bg-primary transition-all duration-300"
												style={{
													width: `${currentPercent}%`,
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
												{current}
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

				{/* Footer Label */}
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
