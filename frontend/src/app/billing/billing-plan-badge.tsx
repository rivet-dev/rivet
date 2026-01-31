import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Badge, cn, Skeleton } from "@/components";
import {
	useCloudDataProvider,
	useCloudProjectDataProvider,
} from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";

const planLabels: Record<string, string> = {
	free: "Free",
	team: "Team",
	pro: "Pro",
	enterprise: "Enterprise",
};

const getPlanVariant = (
	plan: string | undefined,
): "secondary" | "premium" | "premium-blue" => {
	if (plan === "team") return "premium-blue";
	if (plan === "pro" || plan === "enterprise") return "premium";
	return "secondary";
};

export function BillingPlanBadge() {
	const dataProvider = useCloudProjectDataProvider();
	const { data, isLoading } = useQuery({
		...dataProvider.currentProjectBillingDetailsQueryOptions(),
	});

	if (isLoading) {
		return <SkeletonBadge />;
	}

	const plan = data?.billing.activePlan || "free";

	return (
		<Badge
			variant={getPlanVariant(plan)}
			className="min-w-12 justify-center my-px"
		>
			{planLabels[plan]}
		</Badge>
	);
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

	const plan = data?.billing.activePlan || "free";

	return (
		<>
			{isLoading || !isVisible ? (
				<SkeletonBadge />
			) : (
				<Badge
					variant={getPlanVariant(plan)}
					className={cn("min-w-12 justify-center my-px", className)}
				>
					{planLabels[plan]}
				</Badge>
			)}
			<VisibilitySensor onChange={() => setIsVisible(true)} />
		</>
	);
}

const SkeletonBadge = () => <Skeleton className="ml-2 h-6 w-12 rounded-full" />;
