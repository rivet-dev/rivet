import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	H2,
} from "@rivet-gg/components";
import { useBilling } from "./billing-context";
import { BillingUsageProgress } from "./billing-usage-progress";

export function BillingUsage() {
	const {
		activePlan,
		credits: { max, used, overage },
	} = useBilling();

	return (
		<>
			<H2 mt="8">Usage</H2>
			<Card>
				<CardHeader>
					<CardTitle>Dynamic Servers</CardTitle>
				</CardHeader>
				<CardContent>
					<BillingUsageProgress
						max={max}
						used={used}
						overage={overage}
						plan={activePlan}
					/>
				</CardContent>
			</Card>
			<Card>
				<CardHeader>
					<CardTitle>Backend (Database)</CardTitle>
				</CardHeader>
				<CardContent>
					<BillingUsageProgress isFree plan={activePlan} />
				</CardContent>
			</Card>
			<Card>
				<CardHeader>
					<CardTitle>Backend (Modules)</CardTitle>
				</CardHeader>
				<CardContent>
					<BillingUsageProgress isFree plan={activePlan} />
				</CardContent>
			</Card>
		</>
	);
}
