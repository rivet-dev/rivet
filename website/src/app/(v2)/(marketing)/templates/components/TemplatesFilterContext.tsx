"use client";

import { createContext, useContext, useState, useMemo, type ReactNode } from "react";
import type { Technology, Tag, Template } from "@/data/templates/shared";
import Fuse from "fuse.js";

interface TemplatesFilterContextValue {
	searchQuery: string;
	setSearchQuery: (query: string) => void;
	selectedTags: Tag[];
	selectedTechnologies: Technology[];
	handleTagToggle: (tag: Tag) => void;
	handleTechnologyToggle: (tech: Technology) => void;
	isTemplateVisible: (template: Template) => boolean;
	hasActiveFilters: boolean;
	clearAllFilters: () => void;
}

const TemplatesFilterContext = createContext<TemplatesFilterContextValue | null>(null);

export function useTemplatesFilter() {
	const context = useContext(TemplatesFilterContext);
	if (!context) {
		throw new Error("useTemplatesFilter must be used within TemplatesFilterProvider");
	}
	return context;
}

interface TemplatesFilterProviderProps {
	children: ReactNode;
	templates: Template[];
}

export function TemplatesFilterProvider({ children, templates }: TemplatesFilterProviderProps) {
	const [searchQuery, setSearchQuery] = useState("");
	const [selectedTags, setSelectedTags] = useState<Tag[]>([]);
	const [selectedTechnologies, setSelectedTechnologies] = useState<Technology[]>([]);

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
	}, [templates]);

	// Compute which templates match the current filters
	const visibleTemplateNames = useMemo(() => {
		let results = templates;

		// Apply fuzzy search if there's a query
		if (searchQuery.trim() !== "") {
			const fuseResults = fuse.search(searchQuery);
			results = fuseResults.map((result) => result.item);
		}

		// Apply tag and technology filters
		results = results.filter((template) => {
			const matchesTags =
				selectedTags.length === 0 ||
				selectedTags.some((tag) => template.tags.includes(tag));

			const matchesTechnologies =
				selectedTechnologies.length === 0 ||
				selectedTechnologies.some((tech) => template.technologies.includes(tech));

			return matchesTags && matchesTechnologies;
		});

		return new Set(results.map((t) => t.name));
	}, [searchQuery, selectedTags, selectedTechnologies, fuse, templates]);

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

	const isTemplateVisible = (template: Template) => {
		return visibleTemplateNames.has(template.name);
	};

	const hasActiveFilters = selectedTags.length > 0 || selectedTechnologies.length > 0;

	const clearAllFilters = () => {
		setSelectedTags([]);
		setSelectedTechnologies([]);
	};

	return (
		<TemplatesFilterContext.Provider
			value={{
				searchQuery,
				setSearchQuery,
				selectedTags,
				selectedTechnologies,
				handleTagToggle,
				handleTechnologyToggle,
				isTemplateVisible,
				hasActiveFilters,
				clearAllFilters,
			}}
		>
			{children}
		</TemplatesFilterContext.Provider>
	);
}
