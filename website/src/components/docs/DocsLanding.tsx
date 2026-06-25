import { Icon } from "@rivet-gg/icons";
import actorsLogo from "@/images/products/actors-logo.svg";

export interface DocsLandingItem {
	title: string;
	href: string;
	icon: any;
	description?: string;
	badge?: string;
}

export interface DocsLandingSection {
	title: string;
	items: DocsLandingItem[];
}

export interface DocsLandingData {
	title: string;
	subtitle?: string;
	// Optional product logo shown above the title in the hero. "actors" renders
	// the static actors logo.
	logo?: "actors";
	sections: DocsLandingSection[];
}

function HeroTitle({
	title,
	logo,
}: {
	title: string;
	logo?: "actors";
}) {
	if (logo === "actors") {
		return (
			<div className="mb-4 flex items-center justify-center gap-3">
				{/* The actors mark is a solid-white SVG, so darken it to ink to read
				    on the light porcelain background. */}
				<img
					src={actorsLogo.src}
					alt=""
					className="h-8 w-auto brightness-0 md:h-9"
				/>
				<h1 className="text-4xl font-medium tracking-tight text-ink">
					{title}
				</h1>
			</div>
		);
	}
	return (
		<h1 className="mb-4 text-4xl font-medium tracking-tight text-ink">
			{title}
		</h1>
	);
}

// Faint grid backdrop for the card illustration area. Masked with a radial fade
// so the grid is strongest behind the icon and dissolves toward the edges. Uses
// ink-tinted hairlines so it reads on the light porcelain background.
const gridStyle = {
	backgroundImage:
		"linear-gradient(to right, rgba(20,22,20,0.05) 1px, transparent 1px), linear-gradient(to bottom, rgba(20,22,20,0.05) 1px, transparent 1px)",
	backgroundSize: "24px 24px",
	maskImage: "radial-gradient(ellipse 60% 60% at 50% 50%, black 20%, transparent 80%)",
	WebkitMaskImage:
		"radial-gradient(ellipse 60% 60% at 50% 50%, black 20%, transparent 80%)",
};

function LandingCard({ item }: { item: DocsLandingItem }) {
	return (
		<a
			href={item.href}
			className="group flex flex-col overflow-hidden rounded-2xl border border-ink/10 bg-white/55 no-underline transition-colors hover:border-ink/25"
		>
			<div className="relative flex h-36 items-center justify-center overflow-hidden border-b border-ink/10">
				<div className="absolute inset-0" style={gridStyle} />
				<Icon
					icon={item.icon}
					className="relative text-6xl text-pine transition-transform duration-200 group-hover:scale-105"
				/>
			</div>
			<div className="flex flex-col gap-1.5 p-5">
				<div className="flex items-center gap-2">
					<span className="font-medium text-ink">{item.title}</span>
					{item.badge && (
						<span className="rounded-full border border-ink/10 bg-ink/[0.06] px-2 py-0.5 text-[11px] font-medium text-ink-soft">
							{item.badge}
						</span>
					)}
				</div>
				{item.description && (
					<p className="text-sm leading-relaxed text-ink-soft">
						{item.description}
					</p>
				)}
			</div>
		</a>
	);
}

export function DocsLanding({ title, subtitle, logo, sections }: DocsLandingData) {
	const showHeaders = sections.length > 1;

	return (
		<div className="mx-auto flex min-h-[calc(100vh-var(--header-height,3.5rem)-0.5rem)] w-full max-w-5xl flex-col justify-center px-2 py-8">
			<header className="mx-auto mb-16 max-w-2xl text-center">
				<HeroTitle title={title} logo={logo} />
				{subtitle && (
					<p className="text-lg leading-relaxed text-ink-soft">{subtitle}</p>
				)}
			</header>
			<div className="flex flex-col gap-14">
				{sections.map((section) => (
					<section key={section.title}>
						{showHeaders && (
							<h2 className="mb-5 text-xs font-semibold uppercase tracking-wider text-ink-faint">
								{section.title}
							</h2>
						)}
						<div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
							{section.items.map((item) => (
								<LandingCard key={item.href} item={item} />
							))}
						</div>
					</section>
				))}
			</div>
		</div>
	);
}
