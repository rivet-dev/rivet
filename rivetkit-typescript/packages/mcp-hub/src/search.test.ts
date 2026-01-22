import { describe, expect, test } from "vitest";
import { createSearchEngine } from "./search";
import type { DocsMetadata, PageRecord, SectionRecord } from "./types";

function createMockMetadata(
	pages: Partial<PageRecord>[] = [],
	sections: Partial<SectionRecord>[] = [],
): DocsMetadata {
	const fullPages: PageRecord[] = pages.map((p, i) => ({
		resource_uri: p.resource_uri ?? `docs://page/page-${i}`,
		slug: p.slug ?? `page-${i}`,
		path: p.path ?? `/docs/page-${i}`,
		canonical_url: p.canonical_url ?? `https://rivet.gg/docs/page-${i}`,
		title: p.title ?? `Page ${i}`,
		description: p.description ?? `Description for page ${i}`,
		product_area: p.product_area ?? null,
		tags: p.tags ?? [],
		version: p.version ?? "1.0",
		lang: p.lang ?? "en",
		updated_at: p.updated_at ?? "2024-01-01T00:00:00Z",
		token_estimate: p.token_estimate ?? 100,
		headings: p.headings ?? [],
		markdown: p.markdown ?? `# Page ${i}\n\nContent`,
		plaintext: p.plaintext ?? `Page ${i} Content`,
		skill: p.skill ?? false,
	}));

	const fullSections: SectionRecord[] = sections.map((s, i) => ({
		resource_uri: s.resource_uri ?? `docs://page/page-0#section=section-${i}`,
		parent_uri: s.parent_uri ?? "docs://page/page-0",
		title: s.title ?? `Section ${i}`,
		anchor: s.anchor ?? `section-${i}`,
		canonical_url:
			s.canonical_url ?? `https://rivet.gg/docs/page-0#section-${i}`,
		snippet: s.snippet ?? `Snippet for section ${i}`,
		content: s.content ?? `Content for section ${i}`,
		updated_at: s.updated_at ?? "2024-01-01T00:00:00Z",
		token_estimate: s.token_estimate ?? 50,
		path: s.path ?? `/docs/page-0#section-${i}`,
		start_line: s.start_line ?? 1,
		end_line: s.end_line ?? 10,
	}));

	return {
		version: {
			content_hash: "test-hash",
			generated_at: "2024-01-01T00:00:00Z",
		},
		pages: fullPages,
		sections: fullSections,
		llms: [],
		llms_full: [],
	};
}

describe("createSearchEngine", () => {
	test("creates search engine from metadata", () => {
		const metadata = createMockMetadata(
			[{ resource_uri: "docs://page/test" }],
			[{ parent_uri: "docs://page/test" }],
		);
		const engine = createSearchEngine(metadata);
		expect(engine).toBeDefined();
		expect(typeof engine.search).toBe("function");
		expect(typeof engine.getSectionsForPage).toBe("function");
	});

	test("throws when section references missing page", () => {
		const metadata = createMockMetadata(
			[],
			[{ parent_uri: "docs://page/nonexistent" }],
		);
		expect(() => createSearchEngine(metadata)).toThrow("Missing page");
	});
});

describe("SearchEngine.search", () => {
	test("returns empty results for empty query", () => {
		const metadata = createMockMetadata(
			[{ resource_uri: "docs://page/test", title: "Test Page" }],
			[
				{
					parent_uri: "docs://page/test",
					title: "Test Section",
					content: "Test content",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("", { limit: 10, mode: "hybrid", offset: 0 });
		expect(results.results).toHaveLength(0);
		expect(results.total).toBe(0);
	});

	test("returns empty results for whitespace-only query", () => {
		const metadata = createMockMetadata(
			[{ resource_uri: "docs://page/test", title: "Test Page" }],
			[
				{
					parent_uri: "docs://page/test",
					title: "Test Section",
					content: "Test content",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("   ", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
		});
		expect(results.results).toHaveLength(0);
	});

	test("finds sections by title match", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/actors",
					title: "Rivet Actors",
					description: "Actor documentation",
				},
			],
			[
				{
					parent_uri: "docs://page/actors",
					title: "Actor Lifecycle",
					content: "Actors have a lifecycle",
					anchor: "lifecycle",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("lifecycle", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
		});
		expect(results.results.length).toBeGreaterThan(0);
		expect(results.results[0]?.section_title).toBe("Actor Lifecycle");
	});

	test("finds sections by content match", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/websockets",
					title: "WebSockets",
					description: "WebSocket guide",
				},
			],
			[
				{
					parent_uri: "docs://page/websockets",
					title: "Getting Started",
					content: "Use WebSocket connections for realtime data",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("realtime", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
		});
		expect(results.results.length).toBeGreaterThan(0);
	});

	test("ranks title matches higher than content matches", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/test",
					title: "Test Page",
					description: "Description",
				},
			],
			[
				{
					parent_uri: "docs://page/test",
					resource_uri: "docs://page/test#section=content-match",
					title: "Other Title",
					content: "This content mentions actor",
					anchor: "content-match",
				},
				{
					parent_uri: "docs://page/test",
					resource_uri: "docs://page/test#section=title-match",
					title: "Actor Guide",
					content: "This is a guide",
					anchor: "title-match",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("actor", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
		});
		expect(results.results.length).toBe(2);
		expect(results.results[0]?.section_title).toBe("Actor Guide");
	});

	test("respects limit parameter", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/test",
					title: "Test",
					description: "Test description",
				},
			],
			[
				{
					parent_uri: "docs://page/test",
					title: "Section 1",
					content: "actor content one",
				},
				{
					parent_uri: "docs://page/test",
					title: "Section 2",
					content: "actor content two",
				},
				{
					parent_uri: "docs://page/test",
					title: "Section 3",
					content: "actor content three",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("actor", {
			limit: 2,
			mode: "hybrid",
			offset: 0,
		});
		expect(results.results).toHaveLength(2);
		expect(results.total).toBe(3);
	});

	test("respects offset parameter", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/test",
					title: "Test",
					description: "Test description",
				},
			],
			[
				{
					parent_uri: "docs://page/test",
					resource_uri: "docs://page/test#section=s1",
					title: "Section AAA",
					content: "actor content",
					anchor: "s1",
				},
				{
					parent_uri: "docs://page/test",
					resource_uri: "docs://page/test#section=s2",
					title: "Section BBB",
					content: "actor content",
					anchor: "s2",
				},
				{
					parent_uri: "docs://page/test",
					resource_uri: "docs://page/test#section=s3",
					title: "Section CCC",
					content: "actor content",
					anchor: "s3",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const fullResults = engine.search("actor", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
		});
		const offsetResults = engine.search("actor", {
			limit: 10,
			mode: "hybrid",
			offset: 1,
		});
		expect(offsetResults.results.length).toBe(fullResults.results.length - 1);
	});

	test("filters by product_area", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/actors",
					title: "Actors",
					description: "Actor docs",
					product_area: "actors",
				},
				{
					resource_uri: "docs://page/storage",
					title: "Storage",
					description: "Storage docs",
					product_area: "storage",
				},
			],
			[
				{
					parent_uri: "docs://page/actors",
					title: "Actor Section",
					content: "actor data content",
				},
				{
					parent_uri: "docs://page/storage",
					title: "Storage Section",
					content: "storage data content",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("data", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
			filters: { product_area: "actors" },
		});
		expect(results.results.length).toBe(1);
		expect(results.results[0]?.title).toBe("Actors");
	});

	test("filters by version", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/v1",
					title: "V1 Docs",
					description: "Version 1",
					version: "v1",
				},
				{
					resource_uri: "docs://page/v2",
					title: "V2 Docs",
					description: "Version 2",
					version: "v2",
				},
			],
			[
				{
					parent_uri: "docs://page/v1",
					title: "V1 Section",
					content: "feature content",
				},
				{
					parent_uri: "docs://page/v2",
					title: "V2 Section",
					content: "feature content",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("feature", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
			filters: { version: "v2" },
		});
		expect(results.results.length).toBe(1);
		expect(results.results[0]?.title).toBe("V2 Docs");
	});

	test("filters by tags", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/guide",
					title: "Guide",
					description: "Guide content",
					tags: ["tutorial", "beginner"],
				},
				{
					resource_uri: "docs://page/reference",
					title: "Reference",
					description: "Reference content",
					tags: ["api", "advanced"],
				},
			],
			[
				{
					parent_uri: "docs://page/guide",
					title: "Guide Section",
					content: "docs content",
				},
				{
					parent_uri: "docs://page/reference",
					title: "Reference Section",
					content: "docs content",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("docs", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
			filters: { tags: ["tutorial"] },
		});
		expect(results.results.length).toBe(1);
		expect(results.results[0]?.title).toBe("Guide");
	});

	test("normalizes semantic mode to hybrid", () => {
		const metadata = createMockMetadata(
			[{ resource_uri: "docs://page/test", title: "Test" }],
			[{ parent_uri: "docs://page/test", title: "Section", content: "test" }],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("test", {
			limit: 10,
			mode: "semantic",
			offset: 0,
		});
		expect(results.modeUsed).toBe("hybrid");
	});

	test("provides why_matched explanation", () => {
		const metadata = createMockMetadata(
			[
				{
					resource_uri: "docs://page/test",
					title: "Actor Management",
					description: "Manage your actors",
				},
			],
			[
				{
					parent_uri: "docs://page/test",
					title: "Actor Section",
					content: "Actor content here",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const results = engine.search("actor", {
			limit: 10,
			mode: "hybrid",
			offset: 0,
		});
		expect(results.results[0]?.why_matched).toBeDefined();
		expect(results.results[0]?.why_matched.length).toBeGreaterThan(0);
	});
});

describe("SearchEngine.getSectionsForPage", () => {
	test("returns sections for a page", () => {
		const metadata = createMockMetadata(
			[{ resource_uri: "docs://page/test" }],
			[
				{
					parent_uri: "docs://page/test",
					resource_uri: "docs://page/test#section=s1",
				},
				{
					parent_uri: "docs://page/test",
					resource_uri: "docs://page/test#section=s2",
				},
			],
		);
		const engine = createSearchEngine(metadata);
		const sections = engine.getSectionsForPage("docs://page/test");
		expect(sections).toHaveLength(2);
	});

	test("returns empty array for unknown page", () => {
		const metadata = createMockMetadata(
			[{ resource_uri: "docs://page/test" }],
			[{ parent_uri: "docs://page/test" }],
		);
		const engine = createSearchEngine(metadata);
		const sections = engine.getSectionsForPage("docs://page/nonexistent");
		expect(sections).toHaveLength(0);
	});

	test("returns a copy of sections array", () => {
		const metadata = createMockMetadata(
			[{ resource_uri: "docs://page/test" }],
			[{ parent_uri: "docs://page/test" }],
		);
		const engine = createSearchEngine(metadata);
		const sections1 = engine.getSectionsForPage("docs://page/test");
		const sections2 = engine.getSectionsForPage("docs://page/test");
		expect(sections1).not.toBe(sections2);
		expect(sections1).toEqual(sections2);
	});
});
