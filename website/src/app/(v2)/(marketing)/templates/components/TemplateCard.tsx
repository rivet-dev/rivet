import type { Template } from "@/data/templates/shared";
import { TECHNOLOGIES } from "@/data/templates/shared";
import Link from "next/link";
import Image from "next/image";
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
				{/* Template Image - 16:9 aspect ratio matches screenshots (see frontend/packages/example-registry/scripts/build/screenshots.ts) */}
				<div className="w-full aspect-video bg-gradient-to-br from-zinc-800 to-zinc-900 relative overflow-hidden">
					<Image
						src={`/examples/${template.name}/image.png`}
						alt={template.displayName}
						fill
						className="object-cover"
						sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
					/>
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
