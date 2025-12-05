"use client";

import type { Technology, Tag } from "@/data/templates/shared";
import { TECHNOLOGIES, TAGS } from "@/data/templates/shared";
import { Icon, faCheck, faPlus } from "@rivet-gg/icons";
import { useTemplatesFilter } from "./TemplatesFilterContext";
import Link from "next/link";

interface TemplatesSidebarProps {
	allTags: Tag[];
	allTechnologies: Technology[];
}

export function TemplatesSidebar({
	allTags,
	allTechnologies,
}: TemplatesSidebarProps) {
	const {
		selectedTags,
		selectedTechnologies,
		handleTagToggle,
		handleTechnologyToggle,
		hasActiveFilters,
		clearAllFilters,
	} = useTemplatesFilter();

	return (
		<aside className="lg:w-64 flex-shrink-0">
			<div className="space-y-6">
				{/* Type (Tags) */}
				<div>
					<h3 className="text-sm font-medium text-white mb-3">Type</h3>
					<div className="space-y-2">
						{allTags.map((tag) => {
							const tagInfo = TAGS.find((t) => t.name === tag);
							const isSelected = selectedTags.includes(tag);
							return (
								<button
									key={tag}
									onClick={() => handleTagToggle(tag)}
									className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-left transition-all ${
										isSelected
											? "bg-white/10 border border-white/30 text-white"
											: "bg-neutral-950 border border-white/10 text-zinc-400 hover:bg-white/10 hover:border-white/20 hover:text-white"
									}`}
								>
									<div
										className={`flex-shrink-0 w-4 h-4 rounded border flex items-center justify-center ${
											isSelected
												? "bg-white border-white"
												: "border-white/20 bg-transparent"
										}`}
									>
										{isSelected && (
											<Icon icon={faCheck} className="text-xs text-black" />
										)}
									</div>
									<span className="text-sm flex-1">
										{tagInfo?.displayName || tag}
									</span>
								</button>
							);
						})}
					</div>
				</div>

				{/* Technology */}
				<div>
					<h3 className="text-sm font-medium text-white mb-3">Technology</h3>
					<div className="space-y-2">
						{allTechnologies.map((tech) => {
							const techInfo = TECHNOLOGIES.find((t) => t.name === tech);
							const isSelected = selectedTechnologies.includes(tech);
							return (
								<button
									key={tech}
									onClick={() => handleTechnologyToggle(tech)}
									className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-left transition-all ${
										isSelected
											? "bg-white/10 border border-white/30 text-white"
											: "bg-neutral-950 border border-white/10 text-zinc-400 hover:bg-white/10 hover:border-white/20 hover:text-white"
									}`}
								>
									<div
										className={`flex-shrink-0 w-4 h-4 rounded border flex items-center justify-center ${
											isSelected
												? "bg-white border-white"
												: "border-white/20 bg-transparent"
										}`}
									>
										{isSelected && (
											<Icon icon={faCheck} className="text-xs text-black" />
										)}
									</div>
									<span className="text-sm flex-1">
										{techInfo?.displayName || tech}
									</span>
								</button>
							);
						})}
					</div>
				</div>

				{/* Clear filters button */}
				{hasActiveFilters && (
					<button
						onClick={clearAllFilters}
						className="w-full px-4 py-2 text-sm font-medium text-zinc-400 hover:text-white bg-neutral-950 hover:bg-white/10 rounded-lg border border-white/10 hover:border-white/20 transition-all"
					>
						Clear all filters
					</button>
				)}

				{/* Submit template button */}
				<Link
					href="/docs/meta/submit-template"
					className="w-full flex items-center justify-center gap-2 px-4 py-2 text-sm font-medium text-zinc-400 hover:text-white bg-neutral-950 hover:bg-white/10 rounded-lg border border-white/10 hover:border-white/20 transition-all"
				>
					<Icon icon={faPlus} className="text-xs" />
					Submit Template
				</Link>
			</div>
		</aside>
	);
}
