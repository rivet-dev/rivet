"use client";

// Import new sections
import { NewHeroSection } from "./sections/NewHeroSection";
import { SocialProofSection } from "./sections/SocialProofSection";
import { ProblemSection } from "./sections/ProblemSection";
import { SolutionSection } from "./sections/SolutionSection";
import { NewFeaturesBento } from "./sections/NewFeaturesBento";
import { NewUseCases } from "./sections/NewUseCases";
import { NewCTASection } from "./sections/NewCTASection";
import { ScrollObserver } from "@/components/ScrollObserver";

export default function IndexPage() {
	return (
		<ScrollObserver>
			<div className="min-h-screen pt-20" style={{ backgroundColor: '#0A0A0A', color: '#FAFAFA' }}>
				{/* Main container */}
				<div className="container mx-auto max-w-6xl px-4 sm:px-6 lg:px-8">
					{/* Hero Section */}
					<NewHeroSection />

					{/* Social Proof */}
					<SocialProofSection />

					{/* The Problem */}
					<ProblemSection />

					{/* The Solution */}
					<SolutionSection />

					{/* Features Bento */}
					<NewFeaturesBento />

					{/* Use Cases */}
					<NewUseCases />

					{/* Final CTA */}
					<NewCTASection />
				</div>
			</div>
		</ScrollObserver>
	);
}
