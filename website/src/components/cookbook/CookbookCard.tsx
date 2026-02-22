import { Icon, faArrowRight } from "@rivet-gg/icons";

export interface CookbookPageCardData {
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

export function CookbookCard({ page }: { page: CookbookPageCardData }) {
	return (
		<a href={page.href} className="group block">
			<div className="rounded-lg border border-white/10 bg-black p-5 transition-all duration-200 group-hover:border-white/20 group-hover:bg-white/[0.02]">
				<div className="flex items-start justify-between gap-3 mb-2">
					<h3 className="text-sm font-medium text-white">{page.title}</h3>
					<Icon
						icon={faArrowRight}
						className="text-zinc-600 group-hover:text-white transition-all duration-200 text-xs flex-shrink-0 mt-0.5"
					/>
				</div>

				<p className="text-xs text-zinc-500 line-clamp-2 mb-3">{page.description}</p>

				{page.templates.length > 0 && (
					<div className="flex flex-wrap gap-1.5">
						{page.templates.slice(0, 3).map((t) => (
							<span
								key={t.name}
								className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] text-zinc-400"
							>
								{t.displayName}
							</span>
						))}
						{page.templates.length > 3 && (
							<span className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] text-zinc-400">
								+{page.templates.length - 3}
							</span>
						)}
					</div>
				)}
			</div>
		</a>
	);
}
