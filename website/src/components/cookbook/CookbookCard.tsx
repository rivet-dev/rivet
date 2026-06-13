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

// Documentary-style pan vectors. Each cover gets one deterministically from
// its slug so the gallery wall drifts in varied directions rather than in
// lockstep. Pan magnitude stays under the scale overshoot so the frame edge
// never shows through.
const KEN_BURNS_DRIFTS = [
	{ x: "3%", y: "-3%", scale: 1.13 },
	{ x: "-3%", y: "-2%", scale: 1.12 },
	{ x: "2%", y: "3%", scale: 1.14 },
	{ x: "-2%", y: "2%", scale: 1.12 },
] as const;

function driftForSlug(slug: string) {
	let hash = 0;
	for (let i = 0; i < slug.length; i++) {
		hash = (hash + slug.charCodeAt(i)) % 9973;
	}
	return KEN_BURNS_DRIFTS[hash % KEN_BURNS_DRIFTS.length];
}

export function CookbookCard({ page }: { page: CookbookPageCardData }) {
	const drift = driftForSlug(page.slug);
	return (
		<a
			href={page.href}
			className="group relative block aspect-[5/7] overflow-hidden rounded-lg border border-ink/15 bg-ink transition-colors hover:border-pine/60 [container-type:inline-size]"
		>
			{page.cover && (
				<>
					<div
						className="ken-burns absolute inset-0"
						style={
							{
								"--kb-x": drift.x,
								"--kb-y": drift.y,
								"--kb-scale": String(drift.scale),
							} as React.CSSProperties
						}
					>
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
