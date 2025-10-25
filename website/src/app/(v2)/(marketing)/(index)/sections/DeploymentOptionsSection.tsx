import Link from "next/link";
import { CopyCommand } from "../components/CopyCommand";

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
					<h2 className="text-4xl sm:text-5xl font-700 text-white mb-6">
						Run It Your Way
					</h2>
					<p className="text-lg sm:text-xl font-500 text-white/60 max-w-2xl mx-auto">
						Deploy Rivet however works best for your team, from local development to production at scale.
					</p>
				</div>

				<div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-12">
					<DeploymentOption
						title="Local Development"
						description="Just an npm package. No CLI or Docker container to install and learn. Get started in seconds with your existing JavaScript toolchain."
					/>

					<DeploymentOption
						title="Rivet Cloud with Bring Your Own Cloud"
						description="Dead simple to connect to your existing cloud infrastructure. We handle the orchestration, you keep control of your data and infrastructure."
					>
						<div className="flex flex-col gap-3 mt-4">
							<Link
								href="https://hub.rivet.gg/sign-in"
								className="inline-flex items-center gap-2 text-white/80 hover:text-white transition-colors text-sm group"
								target="_blank"
								rel="noopener noreferrer"
							>
								Sign In with Rivet
								<span className="transition-transform group-hover:translate-x-1">→</span>
							</Link>
							<Link
								href="/pricing"
								className="inline-flex items-center gap-2 text-white/80 hover:text-white transition-colors text-sm group"
							>
								View Pricing
								<span className="transition-transform group-hover:translate-x-1">→</span>
							</Link>
						</div>
					</DeploymentOption>

					<DeploymentOption
						title="Self-Host"
						description="Scalable self-hosted deployments. Just a single Docker container or Rust binary connected with Postgres, FoundationDB, or filesystem."
					>
						<div className="mt-4">
							<Link
								href="/docs/general/self-hosting"
								className="inline-flex items-center gap-2 text-white/80 hover:text-white transition-colors text-sm mb-4 group"
							>
								View Self-Hosting Docs
								<span className="transition-transform group-hover:translate-x-1">→</span>
							</Link>
							<div className="mt-3">
								<CopyCommand command="docker run -p 6420:6420 rivetkit/engine" />
							</div>
						</div>
					</DeploymentOption>
				</div>

				<div className="text-center p-6 border border-white/10 rounded-xl bg-white/[0.01]">
					<h4 className="text-lg font-semibold text-white mb-2">Hybrid Deployment</h4>
					<p className="text-white/60">
						Run in Rivet Cloud for production, use self-hosting for on-premises deployments
					</p>
				</div>
			</div>
		</section>
	);
}
