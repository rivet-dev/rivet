import { PlatformIcons } from "../components/PlatformIcons";
import { MarketingButton } from "../components/MarketingButton";
import { CopyCommand } from "../components/CopyCommand";
import Link from "next/link";

export function HeroSection() {
	return (
		<div className="relative isolate mt-[73px] landing-hero flex flex-col px-4 sm:px-6">
			<div className="h-16 sm:h-20" />

			<div className="mx-auto md:px-8 flex flex-col h-full">
				{/* Main content centered vertically */}
				<div className="flex-grow flex flex-col justify-center">
					<div className="max-w-7xl mx-auto text-center">
						{/* Title */}
						<h1 className="hero-bg-exclude text-4xl md:text-5xl font-normal text-white leading-[1.3] sm:leading-[1.1] tracking-[-0.03em] max-w-full">
							{/*Lightweight library for building modern backends*/}
							{/*Library for building stateful applications and distributed systems*/}
							Build and scale stateful workloads
						</h1>

						<div className="h-5" />

						<p className="hero-bg-exclude max-w-3xl text-lg sm:text-xl leading-7 font-light text-white/40 mx-auto transition-colors duration-200">
							Rivet is a library for long-lived processes with{" "}
							<span className="text-white/90">durable state</span>
							, <span className="text-white/90">realtime</span>,
							and{" "}
							<span className="text-white/90">scalability</span>.
							<br />
							Easily{" "}
							<span className="text-white/90">self-hostable</span>{" "}
							and works with{" "}
							<span className="text-white/90">
								your infrastructure
							</span>
							.
						</p>

						<div className="h-8" />

						{/* Libraries Grid */}
						{/*<div className="w-full max-w-4xl mx-auto mb-10 libraries-grid">
							<LibrariesGrid />
						</div>*/}

						{/* CTA Buttons */}
						<div className="hero-bg-exclude flex flex-col sm:flex-row items-center justify-center gap-4">
							<MarketingButton
								href="/docs/actors/quickstart"
								primary
							>
								Quickstart — 5 minutes
							</MarketingButton>

							<MarketingButton href="/talk-to-an-engineer">
								Talk to an engineer
							</MarketingButton>
						</div>

						{/*<div className="h-1" />

						<CopyCommand
							command="npm install @rivetkit/actors"
							className="hero-bg-exclude"
						/>*/}
					</div>
				</div>

				<div className="h-8" />

				<PlatformIcons />

				<div className="h-8 sm:h-12" />
			</div>
		</div>
	);
}
