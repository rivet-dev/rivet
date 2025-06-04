import {
	Badge,
	Button,
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
	H4,
} from "@rivet-gg/components";
import { BillingUsageProgress } from "./billing-usage-progress";
import { useBilling } from "./billing-context";
import { BillingHeader } from "./billing-header";
import { BillingPortalButton } from "./billing-portal-button";
import {
	BillingPlanDescription,
	BillingPlanLead,
	BillingPlanPrice,
	ProjectBillingPlanLabel,
} from "./billing-plan-badge";
import { Link } from "@tanstack/react-router";
import { faArrowRight, Icon } from "@rivet-gg/icons";

export function ActorsBilling() {
	const {
		credits: { max, used, overage },
		activePlan,
		plan,
		group,
		subscription,
	} = useBilling();

	return (
		<div className="max-w-4xl w-full mx-auto px-4">
			<BillingHeader
				actions={
					<>
						<BillingPortalButton
							variant="outline"
							intent="payment_method_update"
						>
							Manage Billing
						</BillingPortalButton>
						<BillingPortalButton variant="outline" intent="general">
							View past invoices
						</BillingPortalButton>
					</>
				}
			/>

			<Card className="bg-transparent mb-4">
				<CardHeader>
					<CardTitle className="flex items-center">
						<div className="flex-1 flex items-center">
							<ProjectBillingPlanLabel plan={plan} /> Plan{" "}
							{activePlan === plan ? (
								<Badge className="ml-2">Current Plan</Badge>
							) : null}
							{activePlan !== plan ? (
								<Badge className="ml-2" variant="secondary">
									Changes on{" "}
									{subscription?.periodEndTs.toLocaleDateString(
										undefined,
										{ dateStyle: "short" },
									)}{" "}
									to{" "}
									<ProjectBillingPlanLabel
										plan={activePlan}
									/>{" "}
									Plan
								</Badge>
							) : null}
						</div>
						<Button
							asChild
							variant="ghost"
							className="text-muted-foreground"
							endIcon={<Icon icon={faArrowRight} />}
						>
							<Link to="." search={{ modal: "manage-plan" }}>
								View Plans
							</Link>
						</Button>
					</CardTitle>
					<CardDescription>
						<BillingPlanPrice plan={plan} />{" "}
						<BillingPlanLead plan={plan} />
					</CardDescription>
				</CardHeader>
				<CardContent className="flex justify-end items-end gap-4">
					<div className="flex-1">
						<p>
							<BillingPlanDescription plan={plan} />
						</p>
						<p>
							Need something custom?{" "}
							<a
								href="https://rivet.gg/sales"
								rel="noopener noreferrer"
								target="_blank"
								className="text-primary"
							>
								Contact Sales
							</a>
						</p>
					</div>
					{plan === activePlan ? (
						<Button asChild>
							<Link to="." search={{ modal: "manage-plan" }}>
								Upgrade
							</Link>
						</Button>
					) : (
						<Button asChild>
							<Link to="." search={{ modal: "manage-plan" }}>
								Change Plan
							</Link>
						</Button>
					)}
				</CardContent>
			</Card>

			<Card className="bg-transparent">
				<CardHeader>
					<CardTitle className="flex items-center">
						<span>Usage</span>
						{subscription ? (
							<Badge
								className="ml-2 font-normal gap-0.5"
								variant="outline"
							>
								<b>
									{subscription.periodStartTs.toLocaleDateString(
										undefined,
										{ dateStyle: "long" },
									)}
								</b>{" "}
								-{" "}
								<b>
									{" "}
									{subscription.periodEndTs.toLocaleDateString(
										undefined,
										{ dateStyle: "long" },
									)}
								</b>{" "}
							</Badge>
						) : null}
					</CardTitle>
				</CardHeader>
				<CardContent>
					<H4 className="font-normal">Actors</H4>
					<BillingUsageProgress
						max={max}
						used={used}
						overage={overage}
						plan={activePlan}
					/>
				</CardContent>
			</Card>
		</div>
	);
}
