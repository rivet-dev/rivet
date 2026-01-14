"use client";

import { templates, type Technology, type Tag, TECHNOLOGIES, TAGS } from "@/data/templates/shared";
import { TemplatesFilterProvider, useTemplatesFilter } from "./TemplatesFilterContext";
import { TemplateCard } from "./TemplateCard";
import { MagnifyingGlassIcon } from "@heroicons/react/24/outline";
import { Icon, faCheck, faPlus } from "@rivet-gg/icons";

interface TemplatesPageContentProps {
	allTags: Tag[];
	allTechnologies: Technology[];
}

function SearchInput() {
	const { searchQuery, setSearchQuery } = useTemplatesFilter();

	return (
		<div className="relative">
			<div className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3">
				<MagnifyingGlassIcon
					className="h-5 w-5 text-zinc-400"
					aria-hidden="true"
				/>
			</div>
			<input
				type="text"
				value={searchQuery}
				onChange={(e) => setSearchQuery(e.target.value)}
				className="block w-full rounded-lg border border-white/20 bg-white/5 pl-10 pr-3 py-3 text-white placeholder:text-zinc-500 focus:border-white/50 focus:outline-none focus:ring-1 focus:ring-white/50 text-base"
				placeholder="Search templates..."
			/>
		</div>
	);
}

function Sidebar({ allTags, allTechnologies }: { allTags: Tag[]; allTechnologies: Technology[] }) {
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
				<a
					href="/docs/meta/submit-template"
					className="w-full flex items-center justify-center gap-2 px-4 py-2 text-sm font-medium text-zinc-400 hover:text-white bg-neutral-950 hover:bg-white/10 rounded-lg border border-white/10 hover:border-white/20 transition-all"
				>
					<Icon icon={faPlus} className="text-xs" />
					Submit Template
				</a>
			</div>
		</aside>
	);
}

function TemplateGrid() {
	const { isTemplateVisible } = useTemplatesFilter();

	const visibleTemplates = templates.filter((template) => isTemplateVisible(template));

	if (visibleTemplates.length === 0) {
		return (
			<div className="text-center py-12 text-zinc-400 col-span-full">
				No templates found matching your filters
			</div>
		);
	}

	return (
		<>
			{visibleTemplates.map((template) => (
				<TemplateCard key={template.name} template={template} />
			))}
		</>
	);
}

function TemplatesContent({ allTags, allTechnologies }: TemplatesPageContentProps) {
	return (
		<>
			<div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-48">
				<div className="mx-auto max-w-[1200px] px-6 lg:px-8">
					<div className="text-center">
						<h1
							className="text-5xl font-medium leading-[1.1] tracking-tighter md:text-7xl"
							style={{ color: "#FAFAFA" }}
						>
							Templates
						</h1>
						<p
							className="mt-6 max-w-2xl mx-auto text-lg md:text-xl leading-[1.2]"
							style={{ color: "#A0A0A0" }}
						>
							Explore Rivet templates and examples to quickly start building
							with Rivet Actors
						</p>
						<div className="mt-8 max-w-md mx-auto">
							<SearchInput />
						</div>
					</div>
				</div>
			</div>

			<div className="relative mx-auto max-w-2xl px-6 lg:max-w-[1400px] lg:px-8 pb-24">
				<div className="flex flex-col lg:flex-row gap-8">
					<Sidebar allTags={allTags} allTechnologies={allTechnologies} />

					<div className="flex-1">
						<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
							<TemplateGrid />
						</div>
					</div>
				</div>
			</div>
		</>
	);
}

export function TemplatesPageContent({ allTags, allTechnologies }: TemplatesPageContentProps) {
	return (
		<TemplatesFilterProvider templates={templates}>
			<TemplatesContent allTags={allTags} allTechnologies={allTechnologies} />
		</TemplatesFilterProvider>
	);
}
