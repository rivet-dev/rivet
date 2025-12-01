import type { Technology, Tag } from "@/data/templates/shared";
import { TECHNOLOGIES, TAGS } from "@/data/templates/shared";

interface TemplatesSidebarProps {
	allTags: Tag[];
	selectedTags: Tag[];
	onTagToggle: (tag: Tag) => void;
	allTechnologies: Technology[];
	selectedTechnologies: Technology[];
	onTechnologyToggle: (tech: Technology) => void;
}

export function TemplatesSidebar({
	allTags,
	selectedTags,
	onTagToggle,
	allTechnologies,
	selectedTechnologies,
	onTechnologyToggle,
}: TemplatesSidebarProps) {
	return (
		<aside className="lg:w-64 flex-shrink-0">
			<div className="sticky top-6 space-y-6">
				{/* Type (Tags) */}
				<div>
					<h3 className="text-sm font-medium text-white mb-3">Type</h3>
					<div className="space-y-2 max-h-64 overflow-y-auto">
						{allTags.map((tag) => {
							const tagInfo = TAGS.find((t) => t.name === tag);
							return (
								<label key={tag} className="flex items-center cursor-pointer group">
									<input
										type="checkbox"
										checked={selectedTags.includes(tag)}
										onChange={() => onTagToggle(tag)}
										className="h-4 w-4 rounded border-white/20 bg-white/5 text-[#FF4500] focus:ring-[#FF4500] focus:ring-offset-0 cursor-pointer"
									/>
									<span className="ml-2 text-sm text-zinc-400 group-hover:text-white transition-colors">
										{tagInfo?.displayName || tag}
									</span>
								</label>
							);
						})}
					</div>
				</div>

				{/* Technology */}
				<div>
					<h3 className="text-sm font-medium text-white mb-3">Technology</h3>
					<div className="space-y-2 max-h-64 overflow-y-auto">
						{allTechnologies.map((tech) => {
							const techInfo = TECHNOLOGIES.find((t) => t.name === tech);
							return (
								<label
									key={tech}
									className="flex items-center cursor-pointer group"
								>
									<input
										type="checkbox"
										checked={selectedTechnologies.includes(tech)}
										onChange={() => onTechnologyToggle(tech)}
										className="h-4 w-4 rounded border-white/20 bg-white/5 text-[#FF4500] focus:ring-[#FF4500] focus:ring-offset-0 cursor-pointer"
									/>
									<span className="ml-2 text-sm text-zinc-400 group-hover:text-white transition-colors">
										{techInfo?.displayName || tech}
									</span>
								</label>
							);
						})}
					</div>
				</div>

				{/* Clear filters button */}
				{(selectedTags.length > 0 || selectedTechnologies.length > 0) && (
					<button
						onClick={() => {
							selectedTags.forEach((tag) => onTagToggle(tag));
							selectedTechnologies.forEach((tech) => onTechnologyToggle(tech));
						}}
						className="w-full px-4 py-2 text-sm font-medium text-zinc-400 hover:text-white bg-white/5 hover:bg-white/10 rounded-lg border border-white/10 hover:border-white/20 transition-all"
					>
						Clear all filters
					</button>
				)}
			</div>
		</aside>
	);
}
