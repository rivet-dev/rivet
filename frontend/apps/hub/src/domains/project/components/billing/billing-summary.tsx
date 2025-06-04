import { AnimatedCurrency, Grid, ValueCard } from "@rivet-gg/components";
import { useBilling } from "./billing-context";

export function BillingSummary() {
	const {
		credits: { total, remaining, max },
	} = useBilling();

	return (
		<Grid columns={{ initial: "1", md: "3" }} gap="4">
			<ValueCard
				title="Current bill total"
				value={<AnimatedCurrency value={total} />}
			/>
			<ValueCard
				title="Credits remaining"
				value={
					<AnimatedCurrency
						from={max}
						value={Math.max(remaining, 0)}
					/>
				}
			/>
		</Grid>
	);
}
