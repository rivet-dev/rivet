import type {
	Metadata,
	PageRecord,
	RankedSection,
	SearchFilters,
	SearchMode,
	SectionRecord,
} from "./types";
import { estimateTokens, stripMarkdown } from "./utils";

type SectionEntry = {
	section: SectionRecord;
	page: PageRecord;
	searchField: string;
	titleField: string;
	descriptionField: string;
	pathField: string;
};

type SearchOptions = {
	filters?: SearchFilters;
	limit: number;
	mode: SearchMode;
	offset: number;
};

export type SearchEngine = {
	search(
		query: string,
		options: SearchOptions,
	): {
		results: RankedSection[];
		modeUsed: SearchMode;
		total: number;
	};
	getSectionsForPage(resourceUri: string): SectionRecord[];
};

export function createSearchEngine(metadata: Metadata): SearchEngine {
	const pageByUri = new Map<string, PageRecord>();
	for (const page of metadata.pages) {
		pageByUri.set(page.resource_uri, page);
	}

	const entries: SectionEntry[] = metadata.sections.map((section) => {
		const page = pageByUri.get(section.parent_uri);
		if (!page) {
			throw new Error(`Missing page for section ${section.resource_uri}`);
		}

		const strippedContent = stripMarkdown(section.content);
		return {
			section,
			page,
			searchField: `${page.title} ${page.description} ${strippedContent}`.toLowerCase(),
			titleField: section.title.toLowerCase(),
			descriptionField: page.description.toLowerCase(),
			pathField: `${page.slug} ${page.path}`.toLowerCase(),
		};
	});

	const sectionsByPage = new Map<string, SectionRecord[]>();
	for (const section of metadata.sections) {
		const list = sectionsByPage.get(section.parent_uri) ?? [];
		list.push(section);
		sectionsByPage.set(section.parent_uri, list);
	}

	const api: SearchEngine = {
		search(query: string, options: SearchOptions) {
			const normalizedQuery = query.trim();
			if (!normalizedQuery) {
				return { results: [], modeUsed: options.mode, total: 0 };
			}

			const tokens = normalizedQuery.toLowerCase().split(/\s+/).filter(Boolean);
			const filter = options.filters;

			const scored = entries
				.map((entry) => {
					if (filter && !matchesFilters(entry.page, filter)) {
						return null;
					}

					const { score, why } = scoreEntry(entry, tokens, normalizedQuery.toLowerCase());
					if (score <= 0) {
						return null;
					}

					const snippet = entry.section.snippet || entry.section.content.slice(0, 200);

					const result: RankedSection = {
						resource_uri: entry.section.resource_uri,
						title: entry.page.title,
						section_title: entry.section.title,
						section_anchor: entry.section.anchor,
						snippet,
						score,
						why_matched: Array.from(new Set(why)).join("; "),
						canonical_url: entry.section.canonical_url,
						updated_at: entry.section.updated_at,
						token_estimate: estimateTokens(entry.section.content),
						path: entry.section.path,
					};
					return result;
				})
				.filter((value): value is RankedSection => Boolean(value))
				.sort((a, b) => {
					if (b.score !== a.score) {
						return b.score - a.score;
					}
					return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
				});

			const paged = scored.slice(options.offset, options.offset + options.limit);
			return {
				results: paged,
				modeUsed: normalizeMode(options.mode),
				total: scored.length,
			};
		},
		getSectionsForPage(resourceUri: string) {
			const list = sectionsByPage.get(resourceUri);
			return list ? [...list] : [];
		},
	};

	return api;
}

function normalizeMode(mode: SearchMode): SearchMode {
	if (mode === "semantic") {
		return "hybrid";
	}
	return mode;
}

function matchesFilters(page: PageRecord, filters: SearchFilters): boolean {
	if (filters.product_area && page.product_area !== filters.product_area) {
		return false;
	}
	if (filters.version && page.version !== filters.version) {
		return false;
	}
	if (filters.lang && page.lang !== filters.lang) {
		return false;
	}
	const filterTags = filters.tags ?? [];
	if (filterTags.length > 0) {
		const pageTags = page.tags as readonly string[];
		const hasAll = filterTags.every((tag) => pageTags.includes(tag));
		if (!hasAll) return false;
	}
	return true;
}

function scoreEntry(entry: SectionEntry, tokens: string[], query: string): { score: number; why: string[] } {
	let score = 0;
	const why = new Set<string>();

	for (const token of tokens) {
		if (!token) continue;
		if (entry.titleField.includes(token)) {
			score += 6;
			why.add(`title contains "${token}"`);
		}
		if (entry.descriptionField.includes(token)) {
			score += 3;
			why.add(`description mentioned "${token}"`);
		}
		if (entry.searchField.includes(token)) {
			score += 4;
			why.add(`content mentions "${token}"`);
		}
		if (entry.pathField.includes(token)) {
			score += 2;
			why.add(`path includes "${token}"`);
		}
	}

	const pageTags = entry.page.tags as readonly string[];
	if (pageTags.some((tag) => query.includes(tag.toLowerCase()))) {
		score += 2;
		why.add("tag overlap");
	}

	if (entry.section.anchor && query.includes(entry.section.anchor)) {
		score += 3;
		why.add("exact anchor match");
	}

	return { score, why: Array.from(why) };
}
