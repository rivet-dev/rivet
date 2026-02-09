import { TalkToAnEngineerForm } from "./form";

export default function TalkToAnEngineerPageClient() {
	return (
		<main className="min-h-screen w-full bg-black selection:bg-[#FF4500]/30 selection:text-orange-200">
			<div className="relative overflow-hidden pt-32 md:pt-48 pb-12">
				<div className="mx-auto max-w-md px-6">
					<h1 className="mb-4 text-4xl font-normal leading-[1.1] tracking-tight text-white text-center whitespace-nowrap">
						Talk to an Engineer
					</h1>
					<p className="mb-10 text-base leading-relaxed text-zinc-500 text-center">
						Connect with one of our engineers to discuss your
						technical needs and how Rivet can help.
					</p>
					<TalkToAnEngineerForm />
				</div>
			</div>
		</main>
	);
}