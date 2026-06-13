import { Icon } from "@rivet-gg/icons";
import { AnimatedAgentOSLogo } from "@/components/marketing/solutions/AgentOSPage";
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
	// Optional product logo shown above the title in the hero. "agentos" renders
	// the animated agentOS logo reused from the marketing page; "actors" renders
	// the static actors logo.
	logo?: "agentos" | "actors";
	sections: DocsLandingSection[];
}

function HeroTitle({
	title,
	logo,
}: {
	title: string;
	logo?: "agentos" | "actors";
}) {
	if (logo === "agentos") {
		// The animated agentOS logo is the wordmark, so it stands in for the title.
		// The source wordmark is black; invert it to white for the dark docs theme.
		return (
			<div className="mb-4 flex items-center justify-center">
				<AnimatedAgentOSLogo className="h-12 w-auto md:h-16 invert" />
			</div>
		);
	}
	if (logo === "actors") {
		return (
			<div className="mb-4 flex items-center justify-center gap-3">
				<img src={actorsLogo.src} alt="" className="h-8 w-auto md:h-9" />
				<h1 className="text-4xl font-semibold tracking-tight text-white">
					{title}
				</h1>
			</div>
		);
	}
	return (
		<h1 className="mb-4 text-4xl font-semibold tracking-tight text-white">
			{title}
		</h1>
	);
}

// Faint grid backdrop for the card illustration area, evoking the line-art
// panels on Mintlify's docs home. Masked with a radial fade so the grid is
// strongest behind the icon and dissolves toward the edges.
const gridStyle = {
	backgroundImage:
		"linear-gradient(to right, rgba(255,255,255,0.06) 1px, transparent 1px), linear-gradient(to bottom, rgba(255,255,255,0.06) 1px, transparent 1px)",
	backgroundSize: "24px 24px",
	maskImage: "radial-gradient(ellipse 60% 60% at 50% 50%, black 20%, transparent 80%)",
	WebkitMaskImage:
		"radial-gradient(ellipse 60% 60% at 50% 50%, black 20%, transparent 80%)",
};

function LandingCard({ item }: { item: DocsLandingItem }) {
	return (
		<a
			href={item.href}
			className="group flex flex-col overflow-hidden rounded-2xl border border-white/10 no-underline transition-colors hover:border-white/25"
		>
			<div className="relative flex h-36 items-center justify-center overflow-hidden border-b border-white/10">
				<div className="absolute inset-0" style={gridStyle} />
				<Icon
					icon={item.icon}
					className="relative text-6xl text-white transition-transform duration-200 group-hover:scale-105"
				/>
			</div>
			<div className="flex flex-col gap-1.5 p-5">
				<div className="flex items-center gap-2">
					<span className="font-semibold text-white">{item.title}</span>
					{item.badge && (
						<span className="rounded-full bg-white/10 px-2 py-0.5 text-[11px] font-medium text-white/60">
							{item.badge}
						</span>
					)}
				</div>
				{item.description && (
					<p className="text-sm leading-relaxed text-white/50">
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
					<p className="text-lg leading-relaxed text-white/50">{subtitle}</p>
				)}
			</header>
			<div className="flex flex-col gap-14">
				{sections.map((section) => (
					<section key={section.title}>
						{showHeaders && (
							<h2 className="mb-5 text-xs font-semibold uppercase tracking-wider text-white/40">
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
