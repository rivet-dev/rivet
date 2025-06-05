import { clusterQueryOptions } from "@/domains/auth/queries/bootstrap";
import { Rivet } from "@rivet-gg/api-ee";
import { Badge, Skeleton } from "@rivet-gg/components";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useIntersectionObserver } from "usehooks-ts";
import { projectBillingQueryOptions } from "../../queries";

const BILLING_PLAN_LABELS = {
	[Rivet.ee.billing.Plan.Indie]: "Pro",
	[Rivet.ee.billing.Plan.Studio]: "Team",
	[Rivet.ee.billing.Plan.Trial]: "Free",
};

const BILLING_PLAN_COLORS = {
	[Rivet.ee.billing.Plan.Indie]: "default",
	[Rivet.ee.billing.Plan.Studio]: "default",
	[Rivet.ee.billing.Plan.Trial]: "secondary",
} as const;

interface BillingPlanBadgeProps {
	projectId: string;
}

function Content({ projectId }: BillingPlanBadgeProps) {
	const { ref, isIntersecting } = useIntersectionObserver({
		root: null,
		rootMargin: "0px",
		threshold: [1],
	});

	const { data, isSuccess } = useQuery(
		projectBillingQueryOptions(projectId, {
			enabled: isIntersecting ?? false,
		}),
	);

	if (!data?.plan) {
		return null;
	}

	if (isSuccess) {
		return (
			<Badge variant={BILLING_PLAN_COLORS[data.plan]}>
				<ProjectBillingPlanLabel plan={data.plan} />
			</Badge>
		);
	}

	return <Skeleton ref={ref} className="w-12 h-6" />;
}

export function BillingPlanBadge({ projectId }: BillingPlanBadgeProps) {
	const { data } = useSuspenseQuery(clusterQueryOptions());

	if (data === "oss") {
		return null;
	}

	return <Content projectId={projectId} />;
}

export function ProjectBillingPlanLabel({
	plan,
}: { plan: Rivet.ee.billing.Plan }) {
	return BILLING_PLAN_LABELS[plan];
}
