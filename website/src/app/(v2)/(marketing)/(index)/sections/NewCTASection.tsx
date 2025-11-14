import Link from "next/link";

export function NewCTASection() {
	return (
		<section className="py-24 md:py-32 text-center">
			<div className="animate-on-scroll animate-fade-up">
				<h2 className="font-heading text-4xl sm:text-5xl font-bold tracking-tighter text-text-primary">
					Build Your Next App with Rivet.
				</h2>

				<p className="mt-6 text-lg md:text-xl text-text-secondary max-w-2xl mx-auto">
					Start in 5 minutes. Deploy anywhere. Scale to millions.
				</p>

				<div className="mt-10 flex flex-col sm:flex-row justify-center gap-4">
					<Link
						href="/docs/quickstart/"
						className="animate-on-scroll animate-fade-up delay-100 px-8 py-4 rounded-lg font-medium bg-accent text-white transition-all duration-200 hover:bg-orange-600 hover:shadow-lg hover:shadow-accent/20 hover:-translate-y-0.5 text-lg"
					>
						Get Started Now
					</Link>
					<Link
						href="/talk-to-an-engineer"
						className="animate-on-scroll animate-fade-up delay-200 px-8 py-4 rounded-lg font-medium border border-border text-text-secondary transition-all duration-200 hover:border-text-secondary hover:text-text-primary text-lg"
					>
						Talk to an Engineer
					</Link>
				</div>
			</div>
		</section>
	);
}
