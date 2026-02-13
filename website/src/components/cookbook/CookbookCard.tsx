import { Icon, faArrowRight, faCode } from "@rivet-gg/icons";

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
		<a href={page.href} className="group block h-full">
			<div className="rounded-lg border border-white/10 bg-black transition-all duration-200 overflow-hidden flex flex-col h-full group-hover:border-white/20">
				<div className="relative">
					<div className="w-full relative overflow-hidden bg-zinc-900">
						<div className="flex items-center gap-2 border-b border-white/5 bg-white/5 px-3 py-2">
							<div className="flex gap-1.5">
								<div className="h-2 w-2 rounded-full bg-zinc-700" />
								<div className="h-2 w-2 rounded-full bg-zinc-700" />
								<div className="h-2 w-2 rounded-full bg-zinc-700" />
							</div>
							<div className="ml-auto text-[10px] text-zinc-500 truncate">
								{page.primaryTemplate?.displayName ?? "Guide"}
							</div>
						</div>
						<div className="aspect-video relative">
							{page.primaryTemplate && !page.primaryTemplate.noFrontend ? (
								<img
									src={`/examples/${page.primaryTemplate.name}/image.png`}
									alt={page.primaryTemplate.displayName}
									width={640}
									height={360}
									className="object-cover absolute inset-0 w-full h-full"
									loading="lazy"
									decoding="async"
								/>
							) : (
								<div className="absolute inset-0 flex items-center justify-center bg-zinc-950">
									<Icon icon={faCode} className="text-zinc-700 text-4xl" />
								</div>
							)}
						</div>
					</div>
				</div>

				<div className="p-4 flex-1 flex flex-col border-t border-white/10">
					<div className="flex items-center justify-between mb-1 gap-4">
						<h3 className="text-sm font-normal text-white flex-1 truncate">{page.title}</h3>
						<Icon
							icon={faArrowRight}
							className="text-zinc-600 group-hover:text-white transition-all duration-200 text-xs flex-shrink-0"
						/>
					</div>

					<p className="text-xs text-zinc-500 line-clamp-2">{page.description}</p>

					{page.templates.length > 0 && (
						<div className="mt-3 flex flex-wrap gap-1.5">
							{page.templates.slice(0, 4).map((t) => (
								<span
									key={t.name}
									className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] text-zinc-300"
								>
									{t.displayName}
								</span>
							))}
							{page.templates.length > 4 && (
								<span className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] text-zinc-400">
									+{page.templates.length - 4}
								</span>
							)}
						</div>
					)}
				</div>
			</div>
		</a>
	);
}
