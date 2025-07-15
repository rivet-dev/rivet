import { CopyCommand } from "../components/CopyCommand";
import { MarketingButton } from "../components/MarketingButton";
import { PlatformIcons } from "../components/PlatformIcons";

export function HeroSection() {
	return (
		<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-40 landing-hero flex flex-col h-screen !justify-start">
			<div className="mx-auto md:px-8 flex flex-col h-full">
				{/* Main content centered vertically */}
				<div className="flex-grow flex flex-col justify-center">
					<div className="max-w-7xl mx-auto text-center">
						{/* Title */}
						<div className="space-y-6">
							<h1 className="hero-bg-exclude text-4xl sm:text-5xl md:text-6xl font-700 text-white leading-[1.3] sm:leading-[1.1] tracking-normal text-6xl leading-[60px] font-normal tracking-[-0.03em] mb-8 inline-block max-w-full pb-0.5">
								{/*Lightweight library for building modern backends*/}
								{/*Library for building stateful applications and distributed systems*/}
								The open-source alternative to Durable Objects
							</h1>
							<p className="hero-bg-exclude max-w-3xl text-xl sm:text-2xl leading-[1.2] tracking-normal font-300 text-white/40 mx-auto text-xl leading-7 text-white/40 font-normal w-max max-w-full mb-10 transition-colors duration-200">
								Rivet Actors is a library that provides{" "}
								<span className="text-white/90">
									durable state
								</span>
								,{" "}
								<span className="text-white/90">realtime</span>,
								and{" "}
								<span className="text-white/90">
									scalability
								</span>
								.<br />
								Easily{" "}
								<span className="text-white/90">
									self-hostable
								</span>{" "}
								and works with{" "}
								<span className="text-white/90">
									your infrastructure
								</span>
								.
							</p>
						</div>

						<div className="h-14" />

						{/* Libraries Grid */}
						{/*<div className="w-full max-w-4xl mx-auto mb-10 libraries-grid">
							<LibrariesGrid />
						</div>*/}

						{/* CTA Buttons */}
						<div className="flex flex-col items-center gap-4 flex gap-4 mb-4 flex-wrap justify-center">
							<div className="hero-bg-exclude flex flex-col sm:flex-row items-center justify-center gap-4">
								<MarketingButton
									href="/docs/actors#getting-started"
									primary
								>
									Get Started
								</MarketingButton>

								<MarketingButton
									href="https://github.com/rivet-gg/rivetkit/tree/main/examples"
									target="_blank"
								>
									See Examples
								</MarketingButton>
							</div>

							<CopyCommand
								command="npm install @rivetkit/actors"
								className="hero-bg-exclude"
							/>
						</div>
					</div>
				</div>

				{/* Platform Icons - Bottom aligned */}
				<div className="w-full max-w-4xl mx-auto mb-8">
					<PlatformIcons />
				</div>
			</div>
		</div>
	);
}
