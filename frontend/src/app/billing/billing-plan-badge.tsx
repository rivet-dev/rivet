import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Badge, cn, Skeleton } from "@/components";
import {
	useCloudDataProvider,
	useCloudProjectDataProvider,
} from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";
import { PLAN_LABELS } from "@/content/billing";

// Tailwind's JIT needs full class strings, so each plan spells out its
// light and dark variants.
const PLAN_COLORS: Record<string, string> = {
	free: "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:border-emerald-500/50 dark:bg-emerald-500/15 dark:text-emerald-300",
	pro: "border-orange-500/40 bg-orange-500/10 text-orange-700 dark:border-orange-500/50 dark:bg-orange-500/15 dark:text-orange-300",
	team: "border-blue-500/40 bg-blue-500/10 text-blue-700 dark:border-blue-500/50 dark:bg-blue-500/15 dark:text-blue-300",
	enterprise:
		"border-purple-500/40 bg-purple-500/10 text-purple-700 dark:border-purple-500/50 dark:bg-purple-500/15 dark:text-purple-300",
	byoc: "border-violet-500/40 bg-violet-500/10 text-violet-700 dark:border-violet-500/50 dark:bg-violet-500/15 dark:text-violet-300",
};

export function PlanBadge({
	plan,
	className,
}: {
	plan: string;
	className?: string;
}) {
	return (
		<Badge
			variant="outline"
			className={cn(
				"shrink-0 justify-center rounded-md px-2 font-mono font-normal leading-4",
				PLAN_COLORS[plan] ?? PLAN_COLORS.free,
				className,
			)}
		>
			{PLAN_LABELS[plan] ?? plan}
		</Badge>
	);
}

export function BillingPlanBadge() {
	const dataProvider = useCloudProjectDataProvider();
	const { data, isLoading } = useQuery({
		...dataProvider.currentProjectBillingDetailsQueryOptions(),
	});

	if (isLoading) {
		return <SkeletonBadge />;
	}

	return <PlanBadge plan={data?.billing.activePlan || "free"} />;
}

export function LazyBillingPlanBadge({
	project,
	organization,
	className,
}: {
	project: string;
	organization: string;
	className?: string;
}) {
	const [isVisible, setIsVisible] = useState(false);
	const dataProvider = useCloudDataProvider();
	const { data, isLoading } = useQuery({
		enabled: isVisible,
		...dataProvider.billingDetailsQueryOptions({ project, organization }),
	});

	return (
		<>
			{isLoading || !isVisible ? (
				<SkeletonBadge />
			) : (
				<PlanBadge
					plan={data?.billing.activePlan || "free"}
					className={className}
				/>
			)}
			<VisibilitySensor onChange={() => setIsVisible(true)} />
		</>
	);
}

const SkeletonBadge = () => <Skeleton className="h-[22px] w-12 rounded-md" />;
