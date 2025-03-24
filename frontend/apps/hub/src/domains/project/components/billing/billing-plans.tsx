import { useDialog } from "@/hooks/use-dialog";
import { Rivet as RivetEe } from "@rivet-gg/api-ee";
import { Flex, Grid, H2, Link } from "@rivet-gg/components";
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
import { BillingPlanStatus } from "./billing-plan-status";

interface BillingPlansProps {
	projectId: string;
	showHeader?: boolean;
	onChoosePlan?: () => Promise<void> | void;
	config?: Partial<
		Record<RivetEe.ee.billing.Plan, Partial<BillingPlanCardProps>>
	>;
}

export function BillingPlans({
	projectId,
	onChoosePlan,
	showHeader = true,
	config,
}: BillingPlansProps) {
	const { dialog, open } = useDialog.ConfirmBillingPlan({
		projectId,
		onSuccess: onChoosePlan,
	});

	const { plan } = useBilling();

	return (
		<>
			{showHeader ? (
				<Flex direction="col" mt="8" mb="4" gap="2">
					<H2>Plan</H2>
					<BillingPlanStatus />
				</Flex>
			) : null}
			{dialog}
			<Grid columns={{ initial: "1", xl: "4" }} gap="4">
				<BillingPlanCard
					title="Community"
					price={`$${PRICE_MAP[RivetEe.ee.billing.Plan.Trial]}`}
					onSubscribe={() =>
						open({
							plan: RivetEe.ee.billing.Plan.Trial,
						})
					}
					onCancel={onChoosePlan}
					type={
						plan === RivetEe.ee.billing.Plan.Trial
							? "active"
							: undefined
					}
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
					{...config?.[RivetEe.ee.billing.Plan.Trial]}
				/>
				<BillingPlanCard
					title="Pro"
					onSubscribe={() =>
						open({
							plan: RivetEe.ee.billing.Plan.Indie,
						})
					}
					onCancel={() =>
						open({
							plan: RivetEe.ee.billing.Plan.Trial,
						})
					}
					price={`$${PRICE_MAP[RivetEe.ee.billing.Plan.Indie]}`}
					type={
						plan === RivetEe.ee.billing.Plan.Indie
							? "active"
							: undefined
					}
					priceLead="+ Actor Usage"
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
					{...config?.[RivetEe.ee.billing.Plan.Indie]}
				/>
				<BillingPlanCard
					title="Team"
					onSubscribe={() =>
						open({
							plan: RivetEe.ee.billing.Plan.Studio,
						})
					}
					onCancel={() =>
						open({
							plan: RivetEe.ee.billing.Plan.Trial,
						})
					}
					price={`$${PRICE_MAP[RivetEe.ee.billing.Plan.Studio]}`}
					type={
						plan === RivetEe.ee.billing.Plan.Studio
							? "active"
							: undefined
					}
					priceLead="+ Actor Usage"
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
					{...config?.[RivetEe.ee.billing.Plan.Studio]}
				/>
				<BillingPlanCard
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
