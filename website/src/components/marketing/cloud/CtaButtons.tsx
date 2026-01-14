import { Icon, faArrowRight } from "@rivet-gg/icons";
import { MarketingButton } from "./MarketingButton";

// Reusable CTA buttons component for use in hero and footer sections
export const CtaButtons = () => {
	return (
		<div className="flex flex-col sm:flex-row items-center sm:items-start gap-4">
			<MarketingButton href="/talk-to-an-engineer" primary>
				Talk to an engineer
			</MarketingButton>
			<MarketingButton href="/docs/cloud">
				<span>Documentation</span>
				<Icon
					icon={faArrowRight}
					className="ml-2 text-xs group-hover:translate-x-0.5 transition-transform duration-200"
				/>
			</MarketingButton>
			{/* TODO */}
			{/*<MarketingButton 
        href="#demo"
      >
        <span>Book Demo</span>
      </MarketingButton>*/}
		</div>
	);
};
