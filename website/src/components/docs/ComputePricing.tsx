import {
	computePricing as defaultModel,
	type ComputePricingModel,
} from "@/data/compute-pricing";

interface ComputePricingProps {
	/** Override the pricing model. Defaults to the shared config. */
	model?: ComputePricingModel;
}

function formatRate(currency: string, rate: number): string {
	// Compute rates are tiny per-second numbers, so render enough precision to
	// stay non-zero without forcing a fixed decimal count on larger values.
	return `${currency}${rate.toPrecision(3)}`;
}

export function ComputePricing({ model = defaultModel }: ComputePricingProps) {
	const ratesReady =
		model.finalized && model.dimensions.every((d) => d.rate !== null);

	if (!ratesReady) {
		return (
			<div className="not-prose rounded-md border border-ink/10 bg-white/55 p-5">
				<p className="text-sm leading-relaxed text-ink-soft">
					Rivet Compute is in beta and per-unit rates are being
					finalized ahead of general availability. You are billed on
					the resources your running instances use:
				</p>
				<ul className="mt-3 space-y-1.5 text-sm text-ink-soft">
					{model.dimensions.map((d) => (
						<li key={d.label}>
							<span className="font-medium text-ink">
								{d.label}
							</span>
							{" — "}
							{d.description}
						</li>
					))}
				</ul>
				<p className="mt-3 text-sm leading-relaxed text-ink-soft">
					{model.billingGranularity} Contact the Rivet team for current
					beta pricing.
				</p>
			</div>
		);
	}

	return (
		<div className="not-prose space-y-4">
			<div className="overflow-x-auto rounded-md border border-ink/10 bg-white/55">
				<table className="w-full text-sm">
					<thead>
						<tr className="border-b border-ink/10 text-left">
							<th className="px-4 py-3 font-medium text-ink">
								Dimension
							</th>
							<th className="px-4 py-3 font-medium text-ink">
								Rate
							</th>
							<th className="px-4 py-3 font-medium text-ink">
								Bills
							</th>
						</tr>
					</thead>
					<tbody>
						{model.dimensions.map((d) => (
							<tr
								key={d.label}
								className="border-b border-ink/10 last:border-0"
							>
								<td className="px-4 py-3 font-medium text-ink">
									{d.label}
								</td>
								<td className="whitespace-nowrap px-4 py-3 font-mono text-ink">
									{formatRate(model.currency, d.rate as number)}{" "}
									<span className="text-ink-faint">
										{d.unit}
									</span>
								</td>
								<td className="px-4 py-3 text-ink-soft">
									{d.description}
								</td>
							</tr>
						))}
					</tbody>
				</table>
			</div>

			{model.includedAllotments.length > 0 ? (
				<div className="rounded-md border border-ink/10 bg-white/55 p-5">
					<p className="text-sm font-medium text-ink">Included each month</p>
					<ul className="mt-2 space-y-1.5 text-sm text-ink-soft">
						{model.includedAllotments.map((a) => (
							<li key={a.label}>
								<span className="font-medium text-ink">
									{a.label}
								</span>
								{" — "}
								{a.amount}
							</li>
						))}
					</ul>
				</div>
			) : null}

			<p className="text-sm leading-relaxed text-ink-soft">
				{model.billingGranularity}
			</p>
		</div>
	);
}
