"use client";

// Import redesigned sections
import { RedesignedHero } from "./sections/RedesignedHero";
import { StatsSection } from "./sections/StatsSection";
import { ConceptSection } from "./sections/ConceptSection";
import { CodeWalkthrough } from "./sections/CodeWalkthrough";
import { ObservabilitySection } from "./sections/ObservabilitySection";
import { FeaturesSection } from "./sections/FeaturesSection";
import { SolutionsSection } from "./sections/SolutionsSection";
import { HostingSection } from "./sections/HostingSection";
import { IntegrationsSection } from "./sections/IntegrationsSection";
import { RedesignedCTA } from "./sections/RedesignedCTA";
import { ScrollObserver } from "@/components/ScrollObserver";

export default function IndexPage() {
	return (
		<ScrollObserver>
			<div className="min-h-screen bg-black text-zinc-300 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
				<main>
					<RedesignedHero />
					<StatsSection />
					<ConceptSection />
					<CodeWalkthrough />
					<FeaturesSection />
					<IntegrationsSection />
					<ObservabilitySection />
					<SolutionsSection />
					<HostingSection />
					<RedesignedCTA />
				</main>
			</div>
		</ScrollObserver>
	);
}
