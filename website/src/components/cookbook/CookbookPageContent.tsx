"use client";

import { useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
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
				<MagnifyingGlassIcon className="h-4 w-4 text-ink-faint" aria-hidden="true" />
			</div>
			<input
				type="text"
				value={value}
				onChange={(e) => onChange(e.target.value)}
				className="block w-full rounded-md border border-ink/15 bg-white/55 pl-10 pr-3 py-2 text-sm text-ink placeholder:text-ink-faint focus:border-pine focus:outline-none transition-colors"
				placeholder="Search guides..."
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
		<div className="flex flex-wrap items-center gap-2">
			<span className="font-mono text-[11px] uppercase tracking-[0.14em] text-ink-faint mr-1">{title}:</span>
			{items.map((item) => {
				const isSelected = selected.includes(item);
				return (
					<button
						key={item}
						onClick={() => onToggle(item)}
						className={`rounded-full border px-3 py-1 text-xs transition-all ${
							isSelected
								? "border-pine bg-pine text-paper"
								: "border-ink/15 text-ink-soft hover:border-ink/30 hover:text-ink"
						}`}
					>
						{getDisplayName(item)}
					</button>
				);
			})}
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
		<div className="paper-grain min-h-screen">
			<div className="pb-24 pt-28 md:pt-36">
				<div className="mx-auto max-w-7xl px-6">
					<h1 className="mb-12 text-center text-5xl font-medium tracking-[-0.015em] text-ink md:mb-16 md:text-6xl">
						Cookbooks
					</h1>
					<div className="mb-10 space-y-4">
						<div className="max-w-sm mx-auto">
							<SearchInput value={searchQuery} onChange={setSearchQuery} />
						</div>

						{(allTags.length > 0 || allTechnologies.length > 0) && (
							<div className="flex flex-wrap items-center justify-center gap-4">
								<FilterChips
									title="Type"
									items={allTags}
									selected={selectedTags}
									onToggle={(t) => toggle(selectedTags, setSelectedTags, t)}
									getDisplayName={(t) => TAGS.find((x) => x.name === t)?.displayName || t}
								/>

								<FilterChips
									title="Tech"
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
										className="text-xs text-ink-faint hover:text-ink transition-colors"
									>
										Clear
									</button>
								)}
							</div>
						)}
					</div>

					{filteredPages.length === 0 ? (
						<div className="text-center py-12 text-ink-soft">No guides found matching your filters</div>
					) : (
						<div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 sm:gap-6">
							<AnimatePresence mode="popLayout" initial={false}>
								{filteredPages.map((page) => (
									<motion.div
										key={page.href}
										layout
										initial={{ opacity: 0, scale: 0.96 }}
										animate={{ opacity: 1, scale: 1 }}
										exit={{ opacity: 0, scale: 0.96 }}
										transition={{ duration: 0.25, ease: "easeOut" }}
									>
										<CookbookCard page={page} />
									</motion.div>
								))}
							</AnimatePresence>
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
