import Link from "next/link";
import { CopyCommand } from "../components/CopyCommand";
import { MarketingButton } from "../components/MarketingButton";

interface DeploymentOptionProps {
	title: string;
	description: string;
	children?: React.ReactNode;
}

function DeploymentOption({ title, description, children }: DeploymentOptionProps) {
	return (
		<div className="border border-white/10 rounded-xl p-8 bg-white/[0.02]">
			<h3 className="text-2xl font-semibold text-white mb-4">{title}</h3>
			<p className="text-white/60 leading-relaxed mb-6">{description}</p>
			{children}
		</div>
	);
}

export function DeploymentOptionsSection() {
	return (
		<section className="w-full">
			<div className="mx-auto max-w-7xl">
				<div className="text-center mb-16">
					<h2 className="text-2xl sm:text-3xl font-700 text-white mb-6">
						Run It Your Way
					</h2>
				</div>

				<div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-12">
					<DeploymentOption
						title="Rivet Cloud"
						description="Build on any cloud while we manage the Actors for you."
					>
						<div className="flex flex-col gap-3 mt-4">
							<Link
								href="/dashboard"
								className="inline-flex items-center gap-2 text-[#FF5C00] hover:text-[#FF5C00]/80 transition-colors text-sm group"
							>
								Sign In with Rivet
								<span className="transition-transform group-hover:translate-x-1">→</span>
							</Link>
						</div>
					</DeploymentOption>

					<DeploymentOption
						title="On-prem/hybrid cloud"
						description="Enterprise grade Rivet for wherever you need it."
					>
						<div className="flex flex-col gap-3 mt-4">
							<Link
								href="/docs/general/self-hosting"
								className="inline-flex items-center gap-2 text-[#FF5C00] hover:text-[#FF5C00]/80 transition-colors text-sm group"
							>
								Contact Sales
								<span className="transition-transform group-hover:translate-x-1">→</span>
							</Link>
						</div>
					</DeploymentOption>

					<DeploymentOption
						title="Rivet Open-Source"
						description="Rivet is open-source Apache 2.0 and easy to build with."
					>
						<div className="flex flex-col gap-3 mt-4">
							<Link
								href="https://github.com/rivet-dev/rivet"
								className="inline-flex items-center gap-2 text-[#FF5C00] hover:text-[#FF5C00]/80 transition-colors text-sm group"
								target="_blank"
								rel="noopener noreferrer"
							>
								Get the source code
								<span className="transition-transform group-hover:translate-x-1">→</span>
							</Link>
						</div>
					</DeploymentOption>
				</div>

				<div className="mt-12">
					<DeploymentOption
						title="Local Development"
						description="Just an npm package. No CLI or Docker container to install and learn. Get started in seconds with your existing JavaScript toolchain."
					>
						<div className="flex flex-col gap-3 mt-4">
							<Link
								href="/docs/getting-started"
								className="inline-flex items-center gap-2 text-[#FF5C00] hover:text-[#FF5C00]/80 transition-colors text-sm group"
							>
								Quickstart
								<span className="transition-transform group-hover:translate-x-1">→</span>
							</Link>
						</div>
					</DeploymentOption>
				</div>
			</div>
		</section>
	);
}
