import { Icon, faCheck, faArrowRight } from "@rivet-gg/icons";
import Link from "next/link";

export default function StartupsPage() {
	return (
		<main className="min-h-screen w-full max-w-[1500px] mx-auto px-4 md:px-8">
			<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-48">
				<div className="mx-auto max-w-[1200px] px-6 lg:px-8">
					<div className="text-center">
						<h1 className="text-4xl md:text-5xl font-normal text-white leading-[1.3] sm:leading-[1.1] tracking-[-0.03em] max-w-full">
							Startup Program
						</h1>
						<div className="h-5" />
						<p className="max-w-3xl text-lg sm:text-xl leading-7 font-light text-white/40 mx-auto">
							Get 50% off Rivet Cloud for your first year. Build stateful, real-time applications with our enterprise-scale actor orchestration platform.
						</p>
					</div>
				</div>
			</div>

			{/* YC Special Discount Banner */}
			<div className="py-16 sm:py-20">
				<div className="max-w-4xl mx-auto px-6 lg:px-8">
					<div className="bg-white/5 border border-white/10 rounded-lg p-6">
						<div className="text-center">
							<h3 className="text-lg font-medium text-white mb-3">
								Special Discount for YC & Speedrun companies
							</h3>
							<p className="text-white/70 mb-4">
								Rivet is a YC W23 and SR002 alum. If you're a current YC/Speedrun company or an alum, you're eligible for additional startup program benefits.
							</p>
							<div className="flex items-center justify-center gap-4 flex-wrap">
								<Link
									href="https://www.ycombinator.com"
									target="_blank"
									className="text-white/80 hover:text-white transition-colors underline decoration-dotted"
								>
									<span className="text-sm font-medium">Bookface Deal</span>
								</Link>
								<span className="text-white/40">•</span>
								<Link
									href="https://speedrun.com"
									target="_blank"
									className="text-white/80 hover:text-white transition-colors underline decoration-dotted"
								>
									<span className="text-sm font-medium">Speedrun Deal</span>
								</Link>
							</div>
						</div>
					</div>
				</div>
			</div>

			<div className="max-w-4xl mx-auto px-6 lg:px-8">

				{/* Benefits */}
				<div className="py-16 sm:py-20">
					<h2 className="text-2xl font-normal mb-8">What you get</h2>
					<div className="grid md:grid-cols-2 gap-6">
						<div className="flex items-start gap-3">
							<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
							<div>
								<h3 className="font-medium mb-2">50% off for 12 months</h3>
								<p className="text-white/70">Get significant savings on Hobby and Team plans during your first year</p>
							</div>
						</div>
						<div className="flex items-start gap-3">
							<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
							<div>
								<h3 className="font-medium mb-2">Enterprise features</h3>
								<p className="text-white/70">Access to FoundationDB backend, multi-region deployment, and advanced security</p>
							</div>
						</div>
						<div className="flex items-start gap-3">
							<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
							<div>
								<h3 className="font-medium mb-2">Priority support</h3>
								<p className="text-white/70">Direct access to our engineering team for technical guidance</p>
							</div>
						</div>
						<div className="flex items-start gap-3">
							<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
							<div>
								<h3 className="font-medium mb-2">Technical consultation</h3>
								<p className="text-white/70">Architecture review and best practices for scaling your application</p>
							</div>
						</div>
					</div>
				</div>

				{/* Requirements */}
				<div className="py-16 sm:py-20">
					<h2 className="text-2xl font-normal mb-8">Requirements</h2>
					<div className="space-y-4">
							<div className="flex items-start gap-3">
								<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
								<div>
									<p className="text-white/70">Less than $5M in venture funding or bootstrapped</p>
								</div>
							</div>
							<div className="flex items-start gap-3">
								<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
								<div>
									<p className="text-white/70">Less than 10 employees</p>
								</div>
							</div>
							<div className="flex items-start gap-3">
								<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
								<div>
									<p className="text-white/70">Incorporated less than 5 years ago</p>
								</div>
							</div>
							<div className="flex items-start gap-3">
								<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
								<div>
									<p className="text-white/70">Privately held & independent</p>
								</div>
							</div>
							<div className="flex items-start gap-3">
								<Icon icon={faCheck} className="text-green-500 mt-1 flex-shrink-0" />
								<div>
									<p className="text-white/70">New Rivet Cloud customer</p>
								</div>
							</div>
					</div>
				</div>

				{/* Application Process */}
				<div className="py-16 sm:py-20">
					<h2 className="text-2xl font-normal mb-8">How to apply</h2>
					<div className="space-y-6">
						<div className="flex gap-4">
							<div className="bg-white/10 rounded-full w-8 h-8 flex items-center justify-center text-sm font-medium flex-shrink-0">
								1
							</div>
							<div>
								<h3 className="font-medium mb-2">Submit your application</h3>
								<p className="text-white/70">Fill out our startup program application with details about your company and use case</p>
							</div>
						</div>
						<div className="flex gap-4">
							<div className="bg-white/10 rounded-full w-8 h-8 flex items-center justify-center text-sm font-medium flex-shrink-0">
								2
							</div>
							<div>
								<h3 className="font-medium mb-2">Review process</h3>
								<p className="text-white/70">Our team will review your application and get back to you within 5 business days</p>
							</div>
						</div>
						<div className="flex gap-4">
							<div className="bg-white/10 rounded-full w-8 h-8 flex items-center justify-center text-sm font-medium flex-shrink-0">
								3
							</div>
							<div>
								<h3 className="font-medium mb-2">Get started</h3>
								<p className="text-white/70">Once approved, you'll receive your discount code and can start building with Rivet Cloud</p>
							</div>
						</div>
					</div>
				</div>

				{/* CTA */}
				<div className="py-24 sm:py-32 text-center">
					<Link
						href="/sales"
						className="inline-flex items-center gap-2 bg-[#FF5C00]/90 hover:bg-[#FF5C00] hover:brightness-110 text-white px-8 py-4 rounded-xl font-medium transition-all duration-200 active:scale-[0.97]"
					>
						Apply for startup program
						<Icon icon={faArrowRight} />
					</Link>
					<p className="text-white/60 mt-4">
						Questions? <Link href="/support" className="text-white hover:underline">Contact us</Link>
					</p>
				</div>
			</div>
		</main>
	);
}
