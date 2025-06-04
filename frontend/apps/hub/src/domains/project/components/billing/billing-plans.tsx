import { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { Grid, Link } from "@rivet-gg/components";
import {
	faBadgeCheck,
	faCheckCircle,
	faClock,
	faComments,
	faDatabase,
	faEnvelope,
	faGift,
	faGlobe,
	faHeadset,
	faInfinity,
	faLockA,
	faPeopleGroup,
	faRocketLaunch,
	faServer,
} from "@rivet-gg/icons";
import { PRICE_MAP } from "../../data/billing-calculate-usage";
import { useBilling } from "./billing-context";
import {
	BillingPlanCard,
	type BillingPlanCardProps,
} from "./billing-plan-card";
import { BillingPlanLead } from "./billing-plan-badge";

interface BillingPlansProps {
	showHeader?: boolean;
	onSubscribe?: (plan: RivetEe.ee.billing.Plan) => void;
	config?: Partial<
		Record<RivetEe.ee.billing.Plan, Partial<BillingPlanCardProps>>
	>;
}

export function BillingPlans({ onSubscribe, config }: BillingPlansProps) {
	const { plan } = useBilling();

	return (
		<>
			<Grid columns={{ initial: "1", xl: "4" }} gap="4">
				<BillingCommunityPlan
					onSubscribe={() =>
						onSubscribe?.(RivetEe.ee.billing.Plan.Trial)
					}
					type={
						plan === RivetEe.ee.billing.Plan.Trial
							? "active"
							: undefined
					}
					{...config?.[RivetEe.ee.billing.Plan.Trial]}
				/>
				<BillingProPlan
					onSubscribe={() =>
						onSubscribe?.(RivetEe.ee.billing.Plan.Indie)
					}
					onCancel={() =>
						onSubscribe?.(RivetEe.ee.billing.Plan.Trial)
					}
					type={
						plan === RivetEe.ee.billing.Plan.Indie
							? "active"
							: undefined
					}
					{...config?.[RivetEe.ee.billing.Plan.Indie]}
				/>
				<BillingTeamPlan
					title="Team"
					onSubscribe={() =>
						onSubscribe?.(RivetEe.ee.billing.Plan.Studio)
					}
					onCancel={() =>
						onSubscribe?.(RivetEe.ee.billing.Plan.Trial)
					}
					type={
						plan === RivetEe.ee.billing.Plan.Studio
							? "active"
							: undefined
					}
					{...config?.[RivetEe.ee.billing.Plan.Studio]}
				/>
				<BillingCustomPlan />
			</Grid>

			<p className="text-center my-4">
				Read more about our plans and see comparison table on our{" "}
				<Link
					href="https://rivet.gg/pricing"
					target="_blank"
					rel="noreferrer"
				>
					pricing page
				</Link>
				.
			</p>
		</>
	);
}

export function BillingCommunityPlan(
	props: Omit<BillingPlanCardProps, "title" | "price" | "features">,
) {
	return (
		<BillingPlanCard
			{...props}
			title="Community"
			price={`$${PRICE_MAP[RivetEe.ee.billing.Plan.Trial]}`}
			features={[
				{
					name: (
						<span>
							$5.00 Free
							<span className="text-xs text-muted-foreground font-normal ml-0.5">
								/mo
							</span>
						</span>
					),
					bold: true,
					icon: faGift,
				},
				{ name: "Community Support", icon: faComments },
			]}
		/>
	);
}

export function BillingProPlan(
	props: Omit<BillingPlanCardProps, "title" | "price" | "features">,
) {
	return (
		<BillingPlanCard
			{...props}
			title="Pro"
			price={`$${PRICE_MAP[RivetEe.ee.billing.Plan.Indie]}`}
			priceLead={<BillingPlanLead plan={RivetEe.ee.billing.Plan.Indie} />}
			features={[
				{
					name: (
						<span>
							$20 Free
							<span className="text-xs text-muted-foreground font-normal ml-0.5">
								/mo
							</span>
						</span>
					),
					bold: true,
					icon: faGift,
				},
				{
					name: "Everything in Community",
					icon: faCheckCircle,
				},
				{ name: "No Usage Limits", icon: faInfinity },

				{ name: "Unlimited Seats", icon: faPeopleGroup },
				{ name: "Email Support", icon: faEnvelope },
			]}
		/>
	);
}

export function BillingTeamPlan(
	props: Omit<BillingPlanCardProps, "title" | "price" | "features">,
) {
	return (
		<BillingPlanCard
			{...props}
			title="Team"
			price={`$${PRICE_MAP[RivetEe.ee.billing.Plan.Studio]}`}
			priceLead={
				<BillingPlanLead plan={RivetEe.ee.billing.Plan.Studio} />
			}
			features={[
				{
					name: (
						<span>
							$200 Free
							<span className="text-xs text-muted-foreground font-normal ml-0.5">
								/mo
							</span>
						</span>
					),
					bold: true,
					icon: faGift,
				},
				{ name: "Everything in Pro", icon: faCheckCircle },
				{
					name: "Dedicated Hardware",
					icon: faServer,
				},
				{ name: "Custom Regions", icon: faGlobe },
				{ name: "Advanced Support", icon: faHeadset },
			]}
		/>
	);
}

export function BillingCustomPlan(
	props: Omit<BillingPlanCardProps, "title" | "price" | "features">,
) {
	return (
		<BillingPlanCard
			{...props}
			title="Enterprise"
			price="Custom"
			features={[
				{ name: "Everything in Team", icon: faCheckCircle },
				{
					name: "Priority Support",
					icon: faHeadset,
				},
				{
					name: "SLA",
					icon: faBadgeCheck,
				},
				{ name: "No Usage Limits", icon: faInfinity },
				{
					name: "OIDC SSO Provider",
					icon: faLockA,
				},
				{
					name: "On-Perm Deployment",
					icon: faRocketLaunch,
				},
				{
					name: "Custom Storage Reads, Writes and Stored Data",
					icon: faDatabase,
				},

				{
					name: "Custom Log Retention",
					icon: faClock,
				},
			]}
			type="custom"
		/>
	);
}
