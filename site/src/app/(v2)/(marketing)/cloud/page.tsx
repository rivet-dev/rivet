import { ArchitectureSection } from "./ArchitectureSection";
import { CommandCenterSection } from "./CommandCenterSection";
import { CtaSection } from "./CtaSection";
import { ServerlessLimitationsSection } from "./ServerlessLimitationsSection";
import { Icon, faArrowRight } from "@rivet-gg/icons";
import { MarketingButton } from "./MarketingButton";
import Link from "next/link";

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
				<ArchitectureSection />
				<ServerlessLimitationsSection />
				{/*<FrameworksSection />*/}
				{/*<TutorialsSection />*/}
				<CommandCenterSection />
				<CtaSection />
			</main>
		</>
	);
}
// Hero component with title, subtitle, and CTA buttons
const Hero = () => {
	return (
		<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-40">
			<div className="mx-auto max-w-[1200px] md:px-8">
				{" "}
				{/* Width/padding ocpied from FancyHeader */}
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
							Scale and manage your Actors on Rivet Cloud
						</h1>
						<p className="text-lg sm:text-xl leading-[1.2] tracking-tight font-500 text-white/40 max-w-lg mx-auto sm:mx-0">
							Rivet cloud scales actors that connect seamlessly to your applications deployed anywhere
						</p>
					</div>

					<div className="h-10" />

					{/* CTA */}
					<div className="flex justify-center sm:justify-start">
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

					{/*<div className="mt-4">
						<p className="text-sm text-white/40 mb-3">or run locally with Docker</p>
						<CopyCommand command="docker run rivetgg/rivet:latest" />
					</div>*/}
				</div>
			</div>
		</div>
	);
};
