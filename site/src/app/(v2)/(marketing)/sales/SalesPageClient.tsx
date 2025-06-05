import { SalesForm } from "./form";

export default function SalesPageClient() {
	return (
		<main className="min-h-screen w-full bg-black">
			<div className="relative isolate overflow-hidden pt-40">
				<div className="mx-auto max-w-4xl px-6 lg:px-8 text-center">
					<h1 className="text-6xl font-700 text-white leading-[1.1] tracking-normal">
						Contact Sales
					</h1>
					<p className="mt-6 max-w-3xl mx-auto text-xl leading-[1.2] tracking-tight font-500 text-white/60">
						Get in touch with our sales team to discuss your
						enterprise needs and how Rivet can help scale your
						infrastructure.
					</p>
				</div>
			</div>

			<div className="mx-auto max-w-2xl px-6 lg:px-8 pt-8 sm:pt-12 pb-16 sm:pb-24">
				<SalesForm />
			</div>
		</main>
	);
}
