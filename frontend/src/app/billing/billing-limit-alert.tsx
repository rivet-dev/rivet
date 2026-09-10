import { faExclamationTriangle, Icon } from "@rivet-gg/icons";
import { skipToken, useQuery } from "@tanstack/react-query";
import { Link, useMatch } from "@tanstack/react-router";
import { AnimatePresence, motion } from "framer-motion";
import { Button, cn } from "@/components";
import { PLAN_LABELS } from "@/content/billing";
import { features } from "@/lib/features";

// Fixed banner height (Tailwind `h-9`). Exported so fixed overlays anchored
// under the top bar (the settings drawer) can start below the banner instead
// of behind it.
export const BILLING_BANNER_HEIGHT = "2.25rem";

export interface BillingLimitBannerState {
	/** The banner is showing: free plan at or above 80% of its included usage. */
	visible: boolean;
	/** Usage has hit or passed 100% of the plan's included allotment. */
	atLimit: boolean;
	plan: string;
	usagePercent: number;
}

/**
 * Resolves the free-plan usage banner state.
 *
 * Both the banner itself and anything that must lay out around it (the
 * settings drawer offsets its top edge by the banner height) call this, so
 * they derive the same answer from the same query data during render. A side
 * channel such as a CSS variable set from an effect would let the two
 * disagree, leaving the banner painted behind the drawer.
 *
 * Billing is project-scoped, but callers render from the shared route layout
 * and the `_context` route, where `useCloudProjectDataProvider` would throw.
 * The provider is read off the project match instead, and the queries are
 * skipped until it resolves, so the hook is safe on every route and keeps a
 * stable tree shape (no outer/inner split that would remount the caller when
 * the project loader lands). Off a project route the banner is hidden.
 */
export function useBillingLimitBanner(): BillingLimitBannerState {
	const projectMatch = useMatch({
		from: "/_context/orgs/$organization/projects/$project",
		shouldThrow: false,
	});
	const dataProvider = features.billing
		? projectMatch?.loaderData?.dataProvider
		: undefined;

	const detailsOptions =
		dataProvider?.currentProjectBillingDetailsQueryOptions();
	const { data: billingData } = useQuery({
		queryKey: detailsOptions?.queryKey ?? ["billing-details", "no-project"],
		queryFn: detailsOptions?.queryFn ?? skipToken,
	});

	const usageOptions = dataProvider?.currentProjectBillingUsageQueryOptions();
	const { data: usage } = useQuery({
		queryKey: usageOptions?.queryKey ?? ["billing-usage", "no-project"],
		queryFn: usageOptions?.queryFn ?? skipToken,
	});

	const usagePercent = usage?.highestPercent ?? 0;
	const plan = billingData?.billing.activePlan || "free";

	return {
		visible: !!dataProvider && plan === "free" && usagePercent >= 80,
		atLimit: usagePercent >= 100,
		plan,
		usagePercent,
	};
}

export function BillingLimitAlert() {
	const { visible, atLimit, plan, usagePercent } = useBillingLimitBanner();

	// The usage figure comes from a slow backend scan, so the banner appears well
	// after the page loads. Expanding it in keeps the content below from jumping.
	return (
		<AnimatePresence>
			{visible ? (
				<motion.div
					initial={{ height: 0, opacity: 0 }}
					animate={{ height: BILLING_BANNER_HEIGHT, opacity: 1 }}
					exit={{ height: 0, opacity: 0 }}
					transition={{ duration: 0.25, ease: "easeOut" }}
					className={cn(
						"overflow-hidden border-b",
						atLimit
							? "border-destructive/60 bg-destructive/15"
							: "border-warning/60 bg-warning/10",
					)}
				>
					<div className="flex h-9 items-center gap-2 px-3 text-xs">
						<Icon
							icon={faExclamationTriangle}
							className={cn(
								"shrink-0",
								atLimit ? "text-destructive" : "text-warning",
							)}
						/>
						<p className="text-foreground font-medium">
							{atLimit
								? `${PLAN_LABELS[plan] ?? "Plan"} plan limit reached`
								: "Approaching your plan limit"}
						</p>
						<p className="text-muted-foreground min-w-0 truncate">
							{atLimit
								? "Upgrade your plan to avoid service interruptions."
								: `You have used ${usagePercent}% of your plan's free usage.`}
						</p>
						<Button
							size="sm"
							variant="ghost"
							className="ml-auto h-6 shrink-0 text-xs"
							asChild
						>
							<Link
								from="/orgs/$organization/projects/$project"
								to="/orgs/$organization/projects/$project/billing"
							>
								Upgrade
							</Link>
						</Button>
					</div>
				</motion.div>
			) : null}
		</AnimatePresence>
	);
}
