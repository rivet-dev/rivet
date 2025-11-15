import { HeroBackground } from "./components/HeroBackground";
import { HeroSection } from "./sections/HeroSection";
import { UseCases } from "./sections/UseCases";
import { PlatformSection } from "./sections/PlatformSection";
import { CodeSnippetsSection } from "./sections/CodeSnippetsSection";
import { FeaturesSection } from "./sections/FeaturesSection";
import { FeaturesBentoBox } from "./sections/FeaturesBentoBox";
import { TechSection } from "./sections/TechSection";
import { DeploymentOptionsSection } from "./sections/DeploymentOptionsSection";
import { StudioSection } from "./sections/StudioSection";
import { CommunitySection } from "./sections/CommunitySection";
import { CTASection } from "./sections/CTASection";

export default function IndexPage() {
	return (
		<>
			<div />

			{/*<HeroBackground />*/}

			{/* Content */}
			<main className="min-h-screen w-full max-w-[1500px] mx-auto px-4 md:px-8">
				<HeroSection />

				<div className="pb-8 sm:pb-12">
					<PlatformSection />
				</div>

				<div className="pb-16 sm:pb-20">
					<UseCases />
				</div>

				<div className="py-24 sm:py-32">
					<CodeSnippetsSection />
				</div>

				<div className="py-24 sm:py-32">
					<FeaturesBentoBox />
				</div>

				{/*<div className="py-24 sm:py-32">
					<FeaturesSection />
				</div>*/}

				<div className="py-24 sm:py-32">
					<TechSection />
				</div>

				<div className="py-24 sm:py-32">
					<DeploymentOptionsSection />
				</div>

				<div className="py-40 sm:py-48">
					<StudioSection />
				</div>

				{/*<div className="py-16 sm:py-20">
				  <QuotesSection />
				</div>*/}

				<div className="py-52 sm:py-60">
					<CommunitySection />
				</div>

				<div className="h-[1px] bg-white/20" />

				<div className="py-52 sm:py-60">
					<CTASection />
				</div>
			</main>
		</>
	);
}
