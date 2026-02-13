"use client";

import { useMemo, useState } from "react";
import { MagnifyingGlassIcon } from "@heroicons/react/24/outline";
import { TECHNOLOGIES, TAGS } from "@/data/templates/shared";
import { CookbookCard, type CookbookPageCardData } from "./CookbookCard";

export interface CookbookPageListItem extends CookbookPageCardData {
	tags: string[];
	technologies: string[];
}

interface CookbookPageContentProps {
	pages: CookbookPageListItem[];
	allTags: string[];
	allTechnologies: string[];
}

function SearchInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
	return (
		<div className="relative">
			<div className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3">
				<MagnifyingGlassIcon className="h-4 w-4 text-zinc-500" aria-hidden="true" />
			</div>
			<input
				type="text"
				value={value}
				onChange={(e) => onChange(e.target.value)}
				className="block w-full rounded-md border border-white/10 bg-black pl-10 pr-3 py-2 text-sm text-white placeholder:text-zinc-600 focus:border-white/20 focus:outline-none transition-colors"
				placeholder="Search..."
			/>
		</div>
	);
}

function FilterChips({
	title,
	items,
	selected,
	onToggle,
	getDisplayName,
}: {
	title: string;
	items: string[];
	selected: string[];
	onToggle: (item: string) => void;
	getDisplayName: (item: string) => string;
}) {
	if (items.length === 0) return null;

	return (
		<div>
			<h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-4">{title}</h3>
			<div className="flex flex-wrap gap-2">
				{items.map((item) => {
					const isSelected = selected.includes(item);
					return (
						<button
							key={item}
							onClick={() => onToggle(item)}
							className={`rounded-full border px-3 py-1 text-xs transition-all ${
								isSelected
									? "border-white/20 text-white bg-white/5"
									: "border-white/10 text-zinc-400 hover:border-white/20 hover:text-white"
							}`}
						>
							{getDisplayName(item)}
						</button>
					);
				})}
			</div>
		</div>
	);
}

export function CookbookPageContent({ pages, allTags, allTechnologies }: CookbookPageContentProps) {
	const [searchQuery, setSearchQuery] = useState("");
	const [selectedTags, setSelectedTags] = useState<string[]>([]);
	const [selectedTechnologies, setSelectedTechnologies] = useState<string[]>([]);

	const filteredPages = useMemo(() => {
		const q = searchQuery.trim().toLowerCase();

		return pages.filter((p) => {
			if (q) {
				const haystack = [
					p.title,
					p.description,
					...p.templates.map((t) => t.displayName),
					...p.templates.map((t) => t.name),
				]
					.join("\n")
					.toLowerCase();
				if (!haystack.includes(q)) return false;
			}

			if (selectedTags.length > 0) {
				const matchesAny = selectedTags.some((t) => p.tags.includes(t));
				if (!matchesAny) return false;
			}

			if (selectedTechnologies.length > 0) {
				const matchesAny = selectedTechnologies.some((t) => p.technologies.includes(t));
				if (!matchesAny) return false;
			}

			return true;
		});
	}, [pages, searchQuery, selectedTags, selectedTechnologies]);

	const hasActiveFilters = selectedTags.length > 0 || selectedTechnologies.length > 0 || searchQuery.trim().length > 0;

	const toggle = (list: string[], setList: (v: string[]) => void, item: string) => {
		setList(list.includes(item) ? list.filter((x) => x !== item) : [...list, item]);
	};

	return (
		<div className="min-h-screen bg-black">
			<div className="relative overflow-hidden pb-12 pt-32 md:pt-48">
				<div className="mx-auto max-w-7xl px-6">
					<div className="max-w-2xl">
						<h1 className="mb-6 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl">
							Cookbook
						</h1>
						<p className="mb-4 text-base leading-relaxed text-zinc-500">
							Step-by-step guides that build on Rivet Actors, with links to working templates and example code.
						</p>
					</div>
				</div>
			</div>

			<div className="border-t border-white/10 py-16">
				<div className="mx-auto max-w-7xl px-6">
					<div className="flex flex-col lg:flex-row gap-12">
						<aside className="lg:w-56 flex-shrink-0">
							<div className="space-y-8">
								<FilterChips
									title="Type"
									items={allTags}
									selected={selectedTags}
									onToggle={(t) => toggle(selectedTags, setSelectedTags, t)}
									getDisplayName={(t) => TAGS.find((x) => x.name === t)?.displayName || t}
								/>

								<FilterChips
									title="Technology"
									items={allTechnologies}
									selected={selectedTechnologies}
									onToggle={(t) => toggle(selectedTechnologies, setSelectedTechnologies, t)}
									getDisplayName={(t) => TECHNOLOGIES.find((x) => x.name === t)?.displayName || t}
								/>

								{hasActiveFilters && (
									<button
										onClick={() => {
											setSearchQuery("");
											setSelectedTags([]);
											setSelectedTechnologies([]);
										}}
										className="text-xs text-zinc-500 hover:text-white transition-colors"
									>
										Clear all filters
									</button>
								)}
							</div>
						</aside>

						<div className="flex-1">
							<div className="mb-8 max-w-md">
								<SearchInput value={searchQuery} onChange={setSearchQuery} />
							</div>

							{filteredPages.length === 0 ? (
								<div className="text-center py-12 text-zinc-400">No guides found matching your filters</div>
							) : (
								<div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
									{filteredPages.map((page) => (
										<CookbookCard key={page.href} page={page} />
									))}
								</div>
							)}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
