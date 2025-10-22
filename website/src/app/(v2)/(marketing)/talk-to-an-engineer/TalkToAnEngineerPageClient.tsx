import { TalkToAnEngineerForm } from "./form";

export default function TalkToAnEngineerPageClient() {
	return (
		<main className="min-h-screen w-full bg-black">
			<div className="relative isolate overflow-hidden pt-40">
				<div className="mx-auto max-w-4xl px-6 lg:px-8 text-center">
					<h1 className="text-6xl font-700 text-white leading-[1.1] tracking-normal">
						Talk to an Engineer
					</h1>
					<p className="mt-6 max-w-3xl mx-auto text-xl leading-[1.2] tracking-tight font-500 text-white/60">
						Connect with one of our engineers to discuss your
						technical needs, current stack, and how Rivet can help
						with your infrastructure challenges.
					</p>
				</div>
			</div>

			<div className="mx-auto max-w-2xl px-6 lg:px-8 pt-8 sm:pt-12 pb-16 sm:pb-24">
				<TalkToAnEngineerForm />
			</div>
		</main>
	);
}