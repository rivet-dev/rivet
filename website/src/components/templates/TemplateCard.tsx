import type { Template } from "@/data/templates/shared";
import { Icon, faArrowRight, faCode } from "@rivet-gg/icons";
import clsx from "clsx";

interface TemplateCardProps {
	template: Template;
}

export function TemplateCard({ template }: TemplateCardProps) {
	// Strip markdown links from description
	const description = template.description.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');

	return (
		<a href={`/templates/${template.name}/`} className="group block h-full">
			<div
				className={clsx(
					"rounded-xl bg-neutral-950 border border-white/20 shadow-sm transition-all duration-200 relative overflow-hidden flex flex-col h-full",
					"group-hover:border-[white]/40 cursor-pointer",
				)}
			>
				{/* Content */}
				<div className="pt-6 px-6 pb-4 flex-1 flex flex-col">
					{/* Title */}
					<div className="flex items-center justify-between mb-2 gap-4">
						<h3 className="text-base font-semibold text-white flex-1 truncate">
							{template.displayName}
						</h3>
						<Icon
							icon={faArrowRight}
							className="text-white/0 group-hover:text-white/60 transition-all duration-200 -translate-x-2 group-hover:translate-x-0 flex-shrink-0"
						/>
					</div>

					{/* Description */}
					<p className="text-sm text-zinc-400 line-clamp-2">
						{description}
					</p>
				</div>

				{/* Template Image - 16:9 aspect ratio matches screenshots (see frontend/packages/example-registry/scripts/build/screenshots.ts) */}
				<div className="px-6 relative">
					<div
						className={clsx(
							"w-full relative overflow-hidden transition-transform duration-200 translate-y-1 group-hover:translate-y-0 bg-gradient-to-br from-zinc-800 to-zinc-900 border-t border-l border-r border-white/20 rounded-t-md",
						)}
					>
						{/* Browser Title Bar */}
						<div className="flex items-center gap-2 border-b border-white/5 bg-white/5 px-2 py-1">
							<div className="flex gap-1">
								<div className="h-1.5 w-1.5 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
								<div className="h-1.5 w-1.5 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
								<div className="h-1.5 w-1.5 rounded-full border border-zinc-500/50 bg-zinc-500/20" />
							</div>
						</div>
						{/* Screenshot Content */}
						<div className="aspect-video relative">
							{!template.noFrontend ? (
								<img
									src={`/examples/${template.name}/image.png`}
									alt={template.displayName}
									className="object-cover absolute inset-0 w-full h-full"
								/>
							) : (
								<div className="absolute inset-0 flex items-center justify-center">
									<Icon
										icon={faCode}
										className="text-zinc-600 text-6xl"
									/>
								</div>
							)}
						</div>
					</div>
					{/* Bottom gradient overlay - stays fixed while screenshot moves */}
					<div className="absolute inset-x-6 bottom-0 h-24 bg-gradient-to-t from-black/60 to-transparent pointer-events-none rounded-t-md overflow-hidden" />
				</div>
			</div>
		</a>
	);
}
