import type { Metadata } from "next";
import Link from "next/link";
import { Button } from "@rivet-gg/components";
import {
	Icon,
	faArrowRight,
} from "@rivet-gg/icons";
import { CopyCommand } from "./CopyCommand";
import { TutorialsSection } from "./TutorialsSection";
import { CommandCenterSection } from "./CommandCenterSection";
import { CommunitySection } from "./CommunitySection";
import { CtaSection } from "./CtaSection";
import { MarketingButton } from "./MarketingButton";
import { CtaButtons } from "./CtaButtons";
import { PowerfulPrimitivesSection } from "./PowerfulPrimitivesSection";
import { ServerlessLimitationsSection } from "./ServerlessLimitationsSection";
import { RivetCloudSection } from "./RivetCloudSection";
import { PerformanceSection } from "./PerformanceSection";
import { FeaturesGrid } from "./FeaturesGrid";
import { FrameworksSection } from "./FrameworksSection";

export const metadata: Metadata = {
	title: "Rivet - The Open-Source Serverless Platform",
	description:
		"Build scalable backends with Rivet—an open-source serverless platform for AI agents, real-time apps, and multiplayer services.",
};

export default function IndexPage() {
	// an empty div at the top of the page is a workaround for a bug in Next.js that causes the page to jump when the user navigates to it
	// https://github.com/vercel/next.js/discussions/64534
	return (
		<>
			<div />

			{/* BG gradient */}
			{/*<div className="absolute inset-0 h-[800px] w-full bg-gradient-to-bl from-[rgba(255,255,255,0.03)] via-[rgba(255,255,255,0.01)] to-transparent z-[-1]"></div>*/}

			{/* Content */}
			<main className="min-h-screen w-full max-w-[1500px] mx-auto px-4 md:px-8">
				<Hero />
				<FeaturesGrid />
				<PowerfulPrimitivesSection />
				<ServerlessLimitationsSection />
				<PerformanceSection />
				{/*<FrameworksSection />*/}
				{/*<TutorialsSection />*/}
				<CommandCenterSection />
				<RivetCloudSection />
				<CommunitySection />
				<CtaSection />
			</main>
		</>
	);
}
// Hero component with title, subtitle, and CTA buttons
const Hero = () => {
	return (
		<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-40"> 
			<div className="mx-auto max-w-[1200px] md:px-8"> {/* Width/padding ocpied from FancyHeader */}
				<div className="max-w-2xl mx-auto sm:mx-0">
					{/* On-Prem CF Workers */}
					{/*<div>
						<Link
							href="/docs/rivet-vs-cloudflare-workers"
							className="group"
						>
							<div className="text-sm px-4 py-2 bg-[#FF5C00]/5 border border-[#FF5C00]/10 rounded-full inline-flex items-center group-hover:bg-[#FF5C00]/10 group-hover:border-[#FF5C00]/20 transition-all">
								<span className="text-white/70">
									Need on-prem{" "}
									<span className="text-white">
										Cloudflare Workers
									</span>{" "}
									or{" "}
									<span className="text-white">
										Durable Objects
									</span>
									?
								</span>
								<Icon
									icon={faArrowRight}
									className="ml-2 text-xs text-[#FF5C00] group-hover:translate-x-0.5 transition-transform"
								/>
							</div>
						</Link>
					</div>

					<div className="h-8" />*/}

					{/* Title */}
					<div className="space-y-6 text-center sm:text-left">
						<h1 className="text-4xl sm:text-5xl md:text-6xl font-700 text-white leading-[1.3] sm:leading-[1.1] tracking-normal">
							The open-source serverless platform
						</h1>
						<p className="text-lg sm:text-xl leading-[1.2] tracking-tight font-500 text-white/40 max-w-lg mx-auto sm:mx-0">
							Easily deploy & scale{" "}
							<span className="text-white/90">AI agents</span>,{" "}
							<span className="text-white/90">
								complex workloads
							</span>, and{" "}
							<span className="text-white/90">backends</span>{" "}
							— all on a frictionless platform that runs anywhere.
							{/*<span className="text-white/80">
								Open-source
							</span> &{" "}
							<span className="text-white/80">
								self-hostable
							</span>.*/}
						</p>
					</div>

					<div className="h-10" />

					{/* CTA */}
					<div className="flex justify-center sm:justify-start">
						<CtaButtons />
					</div>

					{/*<div className="mt-4">
						<p className="text-sm text-white/40 mb-3">or run locally with Docker</p>
						<CopyCommand command="docker run rivetgg/rivet:latest" />
					</div>*/}
				</div>
			</div>
		</div>
	);
};