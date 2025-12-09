"use client";

import { useState, useMemo } from "react";
import { templates, type Technology, type Tag } from "@/data/templates/shared";
import { TemplateCard } from "./components/TemplateCard";
import { TemplatesSidebar } from "./components/TemplatesSidebar";
import { MagnifyingGlassIcon } from "@heroicons/react/24/outline";
import Fuse from "fuse.js";

export default function TemplatesPageClient() {
	const [searchQuery, setSearchQuery] = useState("");
	const [selectedTags, setSelectedTags] = useState<Tag[]>([]);
	const [selectedTechnologies, setSelectedTechnologies] = useState<Technology[]>([]);

	// Get unique tags and technologies from all templates
	const allTags = useMemo(() => {
		const tagsSet = new Set<Tag>();
		templates.forEach((template) => {
			template.tags.forEach((tag) => tagsSet.add(tag));
		});
		return Array.from(tagsSet).sort();
	}, []);

	const allTechnologies = useMemo(() => {
		const techSet = new Set<Technology>();
		templates.forEach((template) => {
			template.technologies.forEach((tech) => techSet.add(tech));
		});
		return Array.from(techSet).sort();
	}, []);

	// Configure Fuse.js for fuzzy searching
	const fuse = useMemo(() => {
		return new Fuse(templates, {
			keys: [
				{ name: "displayName", weight: 2 },
				{ name: "description", weight: 1.5 },
				{ name: "tags", weight: 1 },
				{ name: "technologies", weight: 1 },
			],
			threshold: 0.4,
			includeScore: true,
		});
	}, []);

	// Filter templates based on search and selections
	const filteredTemplates = useMemo(() => {
		let results = templates;

		// Apply fuzzy search if there's a query
		if (searchQuery.trim() !== "") {
			const fuseResults = fuse.search(searchQuery);
			results = fuseResults.map((result) => result.item);
		}

		// Apply tag and technology filters
		results = results.filter((template) => {
			// Tags filter
			const matchesTags =
				selectedTags.length === 0 ||
				selectedTags.some((tag) => template.tags.includes(tag));

			// Technologies filter
			const matchesTechnologies =
				selectedTechnologies.length === 0 ||
				selectedTechnologies.some((tech) => template.technologies.includes(tech));

			return matchesTags && matchesTechnologies;
		});

		return results;
	}, [searchQuery, selectedTags, selectedTechnologies, fuse]);

	const handleTagToggle = (tag: Tag) => {
		setSelectedTags((prev) =>
			prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag],
		);
	};

	const handleTechnologyToggle = (tech: Technology) => {
		setSelectedTechnologies((prev) =>
			prev.includes(tech) ? prev.filter((t) => t !== tech) : [...prev, tech],
		);
	};

	return (
		<main className="min-h-screen w-full max-w-[1500px] mx-auto md:px-8 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
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
							Explore RivetKit templates and examples to quickly start building
							with Rivet Actors
						</p>
						<div className="mt-8 max-w-md mx-auto">
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
									className="block w-full rounded-lg border border-white/20 bg-white/5 pl-10 pr-3 py-3 text-white placeholder:text-zinc-500 focus:border-[#FF4500] focus:outline-none focus:ring-1 focus:ring-[#FF4500] text-base"
									placeholder="Search templates..."
								/>
							</div>
						</div>
					</div>
				</div>
			</div>

			<div className="relative mx-auto max-w-2xl px-6 lg:max-w-[1400px] lg:px-8 pb-24">
				<div className="flex flex-col lg:flex-row gap-8">
					{/* Sidebar */}
					<TemplatesSidebar
						allTags={allTags}
						selectedTags={selectedTags}
						onTagToggle={handleTagToggle}
						allTechnologies={allTechnologies}
						selectedTechnologies={selectedTechnologies}
						onTechnologyToggle={handleTechnologyToggle}
					/>

					{/* Main Grid */}
					<div className="flex-1">
						{filteredTemplates.length === 0 ? (
							<div className="text-center py-12 text-zinc-400">
								No templates found matching your filters
							</div>
						) : (
							<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
								{filteredTemplates.map((template) => (
									<TemplateCard key={template.name} template={template} />
								))}
							</div>
						)}
					</div>
				</div>
			</div>
		</main>
	);
}
