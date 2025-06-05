import { faBook, faComments, faEnvelope, Icon } from "@rivet-gg/icons";
import { Metadata } from "next";

export const metadata: Metadata = {
	title: "Support - Rivet",
	description:
		"Get help with Rivet, from troubleshooting to feature requests.",
};

export default function SupportPage() {
	return (
		<main className="min-h-screen w-full bg-black flex flex-col items-center justify-center">
			<div className="relative isolate overflow-hidden pt-40">
				<div className="mx-auto max-w-4xl px-6 lg:px-8 text-center">
					<h1 className="text-6xl font-700 text-white leading-[1.1] tracking-normal">
						Support
					</h1>
					<p className="mt-6 max-w-3xl mx-auto text-xl leading-[1.2] tracking-tight font-500 text-white/60">
						Get help with Rivet, from troubleshooting to feature
						requests.
					</p>
				</div>
			</div>

			<div className="mx-auto max-w-2xl px-6 lg:px-8 pt-8 sm:pt-12 pb-16 sm:pb-24">
				<div className="grid grid-cols-1 sm:grid-cols-3 gap-6">
					{/* Email Tile */}
					<a
						href="mailto:support@rivet.gg"
						className="group rounded-xl bg-white/2 border border-white/20 hover:border-white/40 shadow-sm transition-all duration-200 relative overflow-hidden h-[180px] flex flex-col items-center justify-center px-8"
					>
						<div className="flex flex-col items-center">
							<Icon
								icon={faEnvelope}
								className="text-3xl text-white mb-2"
							/>
							<span className="font-semibold text-white text-lg mb-1">
								Email
							</span>
							<span className="text-white/60 text-sm">
								support@rivet.gg
							</span>
						</div>
					</a>
					{/* Discord Tile */}
					<a
						href="https://rivet.gg/discord"
						target="_blank"
						rel="noopener noreferrer"
						className="group rounded-xl bg-white/2 border border-white/20 hover:border-white/40 shadow-sm transition-all duration-200 relative overflow-hidden h-[180px] flex flex-col items-center justify-center px-8"
					>
						<div className="flex flex-col items-center">
							<Icon
								icon={faComments}
								className="text-3xl text-white mb-2"
							/>
							<span className="font-semibold text-white text-lg mb-1">
								Discord
							</span>
							<span className="text-white/60 text-sm">
								rivet.gg/discord
							</span>
						</div>
					</a>
					{/* Docs Tile */}
					<a
						href="https://rivet.gg/docs"
						target="_blank"
						rel="noopener noreferrer"
						className="group rounded-xl bg-white/2 border border-white/20 hover:border-white/40 shadow-sm transition-all duration-200 relative overflow-hidden h-[180px] flex flex-col items-center justify-center px-8"
					>
						<div className="flex flex-col items-center">
							<Icon
								icon={faBook}
								className="text-3xl text-white mb-2"
							/>
							<span className="font-semibold text-white text-lg mb-1">
								Docs
							</span>
							<span className="text-white/60 text-sm">
								rivet.gg/docs
							</span>
						</div>
					</a>
				</div>
			</div>
		</main>
	);
}
