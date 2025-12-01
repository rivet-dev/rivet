import type { Template } from "@/data/templates/shared";
import { TECHNOLOGIES } from "@/data/templates/shared";
import Link from "next/link";
import clsx from "clsx";

interface TemplateCardProps {
	template: Template;
}

export function TemplateCard({ template }: TemplateCardProps) {
	return (
		<Link href={`/templates/${template.name}`} className="group block h-full">
			<div
				className={clsx(
					"rounded-xl bg-white/2 border border-white/20 shadow-sm transition-all duration-200 relative overflow-hidden flex flex-col h-full",
					"group-hover:border-[white]/40 cursor-pointer",
				)}
			>
				{/* Placeholder Image */}
				<div className="w-full h-48 bg-gradient-to-br from-zinc-800 to-zinc-900 flex items-center justify-center text-zinc-600">
					<svg
						className="w-16 h-16"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							strokeLinecap="round"
							strokeLinejoin="round"
							strokeWidth={1.5}
							d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
						/>
					</svg>
				</div>

				{/* Content */}
				<div className="flex-1 flex flex-col p-6">
					{/* Title */}
					<h3 className="text-lg font-semibold text-white mb-2 group-hover:text-[#FF4500] transition-colors">
						{template.displayName}
					</h3>

					{/* Description */}
					<p className="text-sm text-zinc-400 mb-4 flex-1 line-clamp-2">
						{template.description}
					</p>

					{/* Technologies */}
					<div className="flex flex-wrap gap-2">
						{template.technologies.map((tech) => {
							const techInfo = TECHNOLOGIES.find((t) => t.name === tech);
							return (
								<span
									key={tech}
									className="inline-flex items-center px-2.5 py-0.5 rounded-md text-xs font-medium bg-white/5 text-zinc-300 border border-white/10"
								>
									{techInfo?.displayName || tech}
								</span>
							);
						})}
					</div>
				</div>
			</div>
		</Link>
	);
}
