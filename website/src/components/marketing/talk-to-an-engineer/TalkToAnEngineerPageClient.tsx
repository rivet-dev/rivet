import { TalkToAnEngineerForm } from "./form";

export default function TalkToAnEngineerPageClient() {
	return (
		<main className="paper-grain min-h-screen w-full">
			<div className="relative overflow-hidden pt-32 md:pt-48 pb-12">
				<div className="mx-auto max-w-md px-6">
					<h1 className="mb-4 text-4xl font-medium leading-[1.06] tracking-[-0.015em] text-ink text-center">
						Talk to an Engineer
					</h1>
					<p className="mb-10 text-base leading-relaxed text-ink-soft text-center">
						Connect with one of our engineers to discuss your
						technical needs and how Rivet can help.
					</p>
					<TalkToAnEngineerForm />
				</div>
			</div>
		</main>
	);
}