import { MarketingButton } from "../components/MarketingButton";
import { AnimatedCTATitle } from "../components/AnimatedCTATitle";

export function CTASection() {
	return (
		<div className="text-center mx-auto max-w-4xl">
			<AnimatedCTATitle />

			<div className="h-8" />

			<div className="flex flex-col sm:flex-row gap-4 justify-center mb-4">
				<MarketingButton 
					href="/docs"
					primary
				>
					Get Started
				</MarketingButton>
				<MarketingButton href="/talk-to-an-engineer">
					Talk to an engineer
				</MarketingButton>
			</div>
		</div>
	);
}
