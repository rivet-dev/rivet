export interface CookbookCardCover {
	src: string;
	objectPosition?: string;
	transform?: string;
	transformOrigin?: string;
	filter?: string;
}

export interface CookbookPageCardData {
	slug: string;
	title: string;
	description: string;
	href: string;
	cover?: CookbookCardCover;
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
		<a
			href={page.href}
			className="group relative block aspect-[5/7] overflow-hidden rounded-lg border border-ink/15 bg-ink transition-colors hover:border-pine/60 [container-type:inline-size]"
		>
			{page.cover && (
				<>
					<div className="absolute inset-0 transition-transform duration-500 ease-out group-hover:scale-[1.02]">
						<img
							src={page.cover.src}
							alt=""
							loading="lazy"
							className="h-full w-full object-cover"
							style={{
								objectPosition: page.cover.objectPosition,
								transform: page.cover.transform,
								transformOrigin: page.cover.transformOrigin,
								filter: page.cover.filter,
							}}
						/>
					</div>
					<div className="absolute inset-0 bg-[linear-gradient(to_bottom,rgba(0,0,0,0.8),rgba(0,0,0,0.3)_32%,transparent_58%)]" />
				</>
			)}
			{/* Title sizes use container-query units so the lockup scales with the card. */}
			<h3
				className="absolute inset-x-0 top-0 px-[8cqw] pt-[12cqw] text-center text-[5.6cqw] font-semibold uppercase leading-[1.55] tracking-[0.2em] text-zinc-50 [text-wrap:balance]"
				style={{ textShadow: "0 2px 16px rgba(0,0,0,0.6)" }}
			>
				{page.title}
			</h3>
		</a>
	);
}
