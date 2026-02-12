"use client";

import { useState } from "react";
import { templates, type Technology, type Tag, TECHNOLOGIES, TAGS } from "@/data/templates/shared";
import { TemplatesFilterProvider, useTemplatesFilter } from "./TemplatesFilterContext";
import { TemplateCard } from "./TemplateCard";
import { MagnifyingGlassIcon } from "@heroicons/react/24/outline";
import { Icon, faCheck, faPlus } from "@rivet-gg/icons";
import { Terminal, Check } from "lucide-react";

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
					className="h-4 w-4 text-zinc-500"
					aria-hidden="true"
				/>
			</div>
			<input
				type="text"
				value={searchQuery}
				onChange={(e) => setSearchQuery(e.target.value)}
				className="block w-full rounded-md border border-white/10 bg-black pl-10 pr-3 py-2 text-sm text-white placeholder:text-zinc-600 focus:border-white/20 focus:outline-none transition-colors"
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
		<aside className="lg:w-56 flex-shrink-0">
			<div className="space-y-8">
				{/* Type (Tags) */}
				<div>
					<h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-4">Type</h3>
					<div className="flex flex-wrap gap-2">
						{allTags.map((tag) => {
							const tagInfo = TAGS.find((t) => t.name === tag);
							const isSelected = selectedTags.includes(tag);
							return (
								<button
									key={tag}
									onClick={() => handleTagToggle(tag)}
									className={`rounded-full border px-3 py-1 text-xs transition-all ${
										isSelected
											? "border-white/20 text-white bg-white/5"
											: "border-white/10 text-zinc-400 hover:border-white/20 hover:text-white"
									}`}
								>
									{tagInfo?.displayName || tag}
								</button>
							);
						})}
					</div>
				</div>

				{/* Technology */}
				<div>
					<h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-4">Technology</h3>
					<div className="flex flex-wrap gap-2">
						{allTechnologies.map((tech) => {
							const techInfo = TECHNOLOGIES.find((t) => t.name === tech);
							const isSelected = selectedTechnologies.includes(tech);
							return (
								<button
									key={tech}
									onClick={() => handleTechnologyToggle(tech)}
									className={`rounded-full border px-3 py-1 text-xs transition-all ${
										isSelected
											? "border-white/20 text-white bg-white/5"
											: "border-white/10 text-zinc-400 hover:border-white/20 hover:text-white"
									}`}
								>
									{techInfo?.displayName || tech}
								</button>
							);
						})}
					</div>
				</div>

				{/* Clear filters button */}
				{hasActiveFilters && (
					<button
						onClick={clearAllFilters}
						className="text-xs text-zinc-500 hover:text-white transition-colors"
					>
						Clear all filters
					</button>
				)}

				{/* Submit template button */}
				<a
					href="/docs/meta/submit-template"
					className="inline-flex items-center justify-center gap-2 rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
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

function CopySkillsButton() {
	const [copied, setCopied] = useState(false);

	const handleCopy = async () => {
		try {
			await navigator.clipboard.writeText('npx skills add rivet-dev/skills');
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch (err) {
			console.error('Failed to copy:', err);
		}
	};

	return (
		<div className="relative group inline-block">
			<button
				onClick={handleCopy}
				className="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white"
			>
				{copied ? <Check className="h-4 w-4 text-green-500" /> : <Terminal className="h-4 w-4" />}
				npx skills add rivet-dev/skills
			</button>
			<div className="absolute left-1/2 -translate-x-1/2 top-full mt-4 opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-200 ease-out text-xs text-zinc-500 whitespace-nowrap pointer-events-none font-mono">
				Give this to your coding agent
			</div>
		</div>
	);
}

function TemplatesContent({ allTags, allTechnologies }: TemplatesPageContentProps) {
	return (
		<div className="min-h-screen bg-black">
			<div className="relative overflow-hidden pb-12 pt-32 md:pt-48">
				<div className="mx-auto max-w-7xl px-6">
					<div className="max-w-2xl">
						<h1 className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl">
							Templates
						</h1>
						<p className="mb-4 text-base leading-relaxed text-zinc-500">
							Explore templates to quickly start building with Rivet Actors, or give this to your coding agent to build something new.
						</p>
						<CopySkillsButton />
					</div>
				</div>
			</div>

			<div className="border-t border-white/10 py-16">
				<div className="mx-auto max-w-7xl px-6">
					<div className="flex flex-col lg:flex-row gap-12">
						<Sidebar allTags={allTags} allTechnologies={allTechnologies} />

						<div className="flex-1">
							<div className="mb-8 max-w-md">
								<SearchInput />
							</div>
							<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
								<TemplateGrid />
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}

export function TemplatesPageContent({ allTags, allTechnologies }: TemplatesPageContentProps) {
	return (
		<TemplatesFilterProvider templates={templates}>
			<TemplatesContent allTags={allTags} allTechnologies={allTechnologies} />
		</TemplatesFilterProvider>
	);
}
