import { SalesForm } from "./form";

export default function SalesPageClient() {
	return (
		<main className="paper-grain min-h-screen w-full">
			<div className="relative overflow-hidden pt-32 md:pt-48 pb-12">
				<div className="mx-auto max-w-md px-6">
					<h1 className="mb-4 text-4xl font-medium leading-[1.06] tracking-[-0.015em] text-ink md:text-5xl text-center">
						Contact Sales
					</h1>
					<p className="mb-10 text-base leading-relaxed text-ink-soft text-center">
						Get in touch with our sales team to discuss your
						enterprise needs and how Rivet can help.
					</p>
					<SalesForm />
				</div>
			</div>
		</main>
	);
}
