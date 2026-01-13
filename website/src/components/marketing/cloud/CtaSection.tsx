"use client";

import { Icon, faArrowRight } from "@rivet-gg/icons";
import { MarketingButton } from "./MarketingButton";

// CTA section
export const CtaSection = () => {
	return (
		<div className="mx-auto max-w-7xl px-6 py-36 lg:py-52 lg:px-8 border-t border-white/5 mt-24">
			<div className="text-center">
				<h2 className="text-4xl font-medium tracking-tight text-white">
					Get building today
				</h2>
				<p className="mt-6 text-xl text-white/70 max-w-lg mx-auto">
					Start for free, no credit card required. Deploy your first
					serverless project in minutes.
				</p>

				<div className="mt-12 flex items-center justify-center gap-x-6">
					<div className="flex flex-col sm:flex-row items-center sm:items-start gap-4">
						<MarketingButton href="/talk-to-an-engineer" primary>
							Get Started
						</MarketingButton>
						<MarketingButton href="/docs/cloud">
							<span>Documentation</span>
							<Icon
								icon={faArrowRight}
								className="ml-2 text-xs group-hover:translate-x-0.5 transition-transform duration-200"
							/>
						</MarketingButton>
					</div>
				</div>
			</div>
		</div>
	);
};
