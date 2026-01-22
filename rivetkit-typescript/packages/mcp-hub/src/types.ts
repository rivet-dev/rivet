export type DocsMetadata = {
	version: {
		content_hash: string;
		generated_at: string;
	};
	pages: PageRecord[];
	sections: SectionRecord[];
	llms: string[];
	llms_full: string[];
};

export type PageRecord = {
	resource_uri: string;
	slug: string;
	path: string;
	canonical_url: string;
	title: string;
	description: string;
	product_area: string | null;
	tags: string[];
	version: string;
	lang: string;
	updated_at: string;
	token_estimate: number;
	headings: Array<{
		anchor: string;
		level: number;
		title: string;
		startLine: number;
		endLine: number;
	}>;
	markdown: string;
	plaintext: string;
	skill: boolean;
};

export type SectionRecord = {
	resource_uri: string;
	parent_uri: string;
	title: string;
	anchor: string;
	canonical_url: string;
	snippet: string;
	content: string;
	updated_at: string;
	token_estimate: number;
	path: string;
	start_line: number;
	end_line: number;
};

export type Metadata = DocsMetadata;

export type SearchMode = "keyword" | "semantic" | "hybrid";

export type SearchFilters = {
	product_area?: string;
	version?: string;
	lang?: string;
	tags?: string[];
};

export type RankedSection = {
	resource_uri: string;
	title: string;
	section_title: string;
	section_anchor?: string;
	snippet: string;
	score: number;
	why_matched: string;
	canonical_url: string;
	updated_at: string;
	token_estimate: number;
	path: string;
};

export type DocsServerOptions = {
	metadata?: DocsMetadata;
	instructions?: string;
};
