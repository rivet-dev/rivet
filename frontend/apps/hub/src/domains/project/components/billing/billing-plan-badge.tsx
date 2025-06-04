import { clusterQueryOptions } from "@/domains/auth/queries/bootstrap";
import { Rivet } from "@rivet-gg/api-ee";
import { Badge, Skeleton } from "@rivet-gg/components";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useIntersectionObserver } from "usehooks-ts";
import { projectBillingQueryOptions } from "../../queries";
import { PRICE_MAP } from "../../data/billing-calculate-usage";

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

const BILLING_PLAN_LEADS = {
	[Rivet.ee.billing.Plan.Indie]: "+ Usage",
	[Rivet.ee.billing.Plan.Studio]: "+ Usage",
	[Rivet.ee.billing.Plan.Trial]: "",
} satisfies Record<Rivet.ee.billing.Plan, string>;

export function BillingPlanLead({
	plan,
}: {
	plan: Rivet.ee.billing.Plan | undefined;
}) {
	if (!plan) {
		return null;
	}

	return BILLING_PLAN_LEADS[plan];
}

export function BillingPlanPrice({
	plan,
}: {
	plan: Rivet.ee.billing.Plan | undefined;
}) {
	if (!plan) {
		return null;
	}

	return <>${PRICE_MAP[plan]} / Month</>;
}

export function BillingPlanDescription({
	plan,
}: {
	plan: Rivet.ee.billing.Plan | undefined;
}) {
	if (!plan) {
		return null;
	}

	switch (plan) {
		case Rivet.ee.billing.Plan.Indie:
			return (
				<p>Unlimited usage, seats, support, and $20.00 in credits.</p>
			);
		case Rivet.ee.billing.Plan.Studio:
			return (
				<p>
					Dedicated hardware, custom regions, priority support,
					unlimited usage, seats, and $200.00 in credits.
				</p>
			);
		case Rivet.ee.billing.Plan.Trial:
			return (
				<p>
					Limited usage, seats, community support, and $5.00 in
					credits
				</p>
			);
		default:
			return null;
	}
}
