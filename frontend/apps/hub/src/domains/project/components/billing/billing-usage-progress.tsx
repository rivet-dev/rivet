import { Rivet as RivetEe } from "@rivet-gg/api-ee";
import {
	Alert,
	AlertTitle,
	Flex,
	RangedProgressBar,
	Text,
	formatCurrency,
} from "@rivet-gg/components";
import { Icon, faExclamationTriangle } from "@rivet-gg/icons";
import { BILLING_PLANS_CREDITS_VISIBILITY as BILLING_PLANS_OVERAGE_ALERT_VISIBILITY } from "../../data/billing-calculate-usage";

// Use a number below 100 to left some space for the overage
const MAX_RANGE = 75;

interface CommonUsageProgressProps {
	plan: RivetEe.ee.billing.Plan;
}

interface FreeUsageProgressProps {
	isFree: true;
}

interface PaidUsageProgressProps {
	isFree?: false;
	max: number;
	used: number;
	overage: number;
}

type BillingUsageProgressProps = CommonUsageProgressProps &
	(FreeUsageProgressProps | PaidUsageProgressProps);

export function BillingUsageProgress(props: BillingUsageProgressProps) {
	if (props.isFree) {
		return (
			<Flex gap="8" justify="center" items="center" py="8">
				<Text my="4">Free while in Beta</Text>
			</Flex>
		);
	}

	const { used, max, overage, plan } = props;
	const percentage = Math.min((used / max) * MAX_RANGE, MAX_RANGE + 10);

	return (
		<>
			{overage > 0 &&
			BILLING_PLANS_OVERAGE_ALERT_VISIBILITY.includes(plan) ? (
				<Alert variant="destructive" className="animate-shake">
					<Icon className="size-4" icon={faExclamationTriangle} />
					<AlertTitle>
						You have exceeded the billing credits limit!
					</AlertTitle>
				</Alert>
			) : null}
			<Flex
				gap="8"
				direction={{ initial: "col", md: "row" }}
				justify="center"
				items="center"
				pb="4"
			>
				<RangedProgressBar
					percentage={percentage}
					maxRange={MAX_RANGE}
					minLabel={
						<>
							Usage
							<br />
							{formatCurrency(used)}
						</>
					}
					maxLabel={
						<>
							{plan === RivetEe.ee.billing.Plan.Trial
								? "Free tier limit"
								: "Included credits"}
							<br />
							{formatCurrency(max)}
						</>
					}
				/>
				<div className="flex-shrink-0 text-center">
					<p className="text-base">Month-to-date</p>
					<p className="text-4xl font-bold">
						{formatCurrency(overage)}
					</p>
				</div>
			</Flex>
		</>
	);
}
