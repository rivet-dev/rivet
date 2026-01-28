import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Badge, Skeleton } from "@/components";
import {
	useCloudDataProvider,
	useCloudProjectDataProvider,
} from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";

const planLabels: Record<string, string> = {
	free: "Free",
	pro: "Pro",
	enterprise: "Enterprise",
};

export function BillingPlanBadge() {
	const dataProvider = useCloudProjectDataProvider();
	const { data, isLoading } = useQuery({
		...dataProvider.currentProjectBillingDetailsQueryOptions(),
	});

	if (isLoading) {
		return <SkeletonBadge />;
	}
	return (
		<Badge variant="secondary">
			{planLabels[data?.billing.activePlan || "free"]}
		</Badge>
	);
}

export function LazyBillingPlanBadge({
	project,
	organization,
}: {
	project: string;
	organization: string;
}) {
	const [isVisible, setIsVisible] = useState(false);
	const dataProvider = useCloudDataProvider();
	const { data, isLoading } = useQuery({
		enabled: isVisible,
		...dataProvider.billingDetailsQueryOptions({ project, organization }),
	});

	if (isLoading) {
		return (
			<>
				<SkeletonBadge />
				<VisibilitySensor onChange={() => setIsVisible(true)} />
			</>
		);
	}
	return (
		<Badge variant="secondary">
			{planLabels[data?.billing.activePlan || "free"]}
		</Badge>
	);
}

const SkeletonBadge = () => <Skeleton className="ml-2 h-6 w-12 rounded-full" />;
