export interface Tag {
	name: string;
	displayName: string;
}

export interface Technology {
	name: string;
	displayName: string;
}

export interface Template {
	name: string;
	displayName: string;
	description: string;
	tags: string[];
	technologies: string[];
	providers: {
		vercel: string | null;
	};
	noFrontend?: boolean;
}

// Local fallback registry for cookbook pages.
// Entries can be expanded as cookbook templates are added.
export const templates: Template[] = [];
export const TAGS: Tag[] = [];
export const TECHNOLOGIES: Technology[] = [];
