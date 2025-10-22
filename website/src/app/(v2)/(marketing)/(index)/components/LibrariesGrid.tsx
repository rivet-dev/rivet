import {
	Icon,
	faArrowRight,
	faShapes,
	faSquareQuestion,
} from "@rivet-gg/icons";
import Link from "next/link";

export function LibrariesGrid() {
	return (
		<div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-4xl mx-auto">
			<Link href="/docs/actors" className="block group">
				<div className="rounded-xl bg-white/2 border border-white/20 group-hover:border-white/40 shadow-sm transition-all duration-200 relative overflow-hidden h-[200px] flex flex-col">
					<div className="px-6 mt-6">
						<div className="flex items-center justify-between mb-4">
							<div className="flex items-center gap-3 text-white text-base">
								<Icon icon={faShapes} />
								<h3 className="font-medium">Actors</h3>
							</div>
							<div className="opacity-0 group-hover:opacity-100 transition-opacity">
								<Icon
									icon={faArrowRight}
									className="text-white/80 text-xl -translate-x-1 group-hover:translate-x-0 transition-all"
								/>
							</div>
						</div>
						<div className="space-y-3">
							<p className="text-white/40 text-base leading-relaxed">
								Long running tasks with state persistence,
								hibernation, and realtime
							</p>
							<p className="text-sm text-white/30">
								Replaces{" "}
								<span className="text-white/60 font-medium">
									Durable Objects
								</span>
								,{" "}
								<span className="text-white/60 font-medium">
									Orleans
								</span>
								, or{" "}
								<span className="text-white/60 font-medium">
									Akka
								</span>
							</p>
						</div>
					</div>
				</div>
			</Link>

			<div className="rounded-xl bg-white/2 border border-white/20 shadow-sm relative overflow-hidden h-[200px] flex flex-col">
				<div className="px-6 mt-6">
					<div className="flex items-center gap-3 mb-4 text-white text-base">
						<Icon icon={faSquareQuestion} />
						<h3 className="font-medium">Coming Soon</h3>
					</div>
					<div className="space-y-3">
						<p className="text-white/40 text-base leading-relaxed">
							Stay tuned for more
						</p>
					</div>
				</div>
			</div>
		</div>
	);
}
