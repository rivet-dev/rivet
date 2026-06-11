import { CookbookCover } from "./CookbookCover";

export interface CookbookPageCardData {
	slug: string;
	title: string;
	description: string;
	href: string;
	primaryTemplate?: {
		name: string;
		displayName: string;
		noFrontend?: boolean;
	};
	templates: Array<{
		name: string;
		displayName: string;
	}>;
}

export function CookbookCard({ page, numeral }: { page: CookbookPageCardData; numeral: string }) {
	return (
		<a
			href={page.href}
			className="group relative block aspect-[5/7] overflow-hidden rounded-[10px] border border-white/10 bg-black transition-colors duration-[400ms] hover:border-white/25 [container-type:inline-size]"
		>
			<CookbookCover slug={page.slug} title={page.title} numeral={numeral} />
		</a>
	);
}
