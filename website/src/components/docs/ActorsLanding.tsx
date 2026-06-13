import {
	Icon,
	faArrowRight,
	faBolt,
	faCloudflare,
	faFloppyDisk,
	faFunction,
	faLayerGroup,
	faNextjs,
	faNodeJs,
	faReact,
	faRust,
	faSqlite,
	faTowerBroadcast,
} from "@rivet-gg/icons";

interface LandingItem {
	title: string;
	description: string;
	href: string;
	icon: any;
	badge?: string;
}

const quickstarts: LandingItem[] = [
	{
		title: "Node.js & Bun",
		description: "Actors with Node.js, Bun, and web frameworks",
		href: "/docs/actors/quickstart/backend",
		icon: faNodeJs,
	},
	{
		title: "React",
		description: "Realtime React applications backed by actors",
		href: "/docs/actors/quickstart/react",
		icon: faReact,
	},
	{
		title: "Next.js",
		description: "Server-rendered Next.js apps backed by actors",
		href: "/docs/actors/quickstart/next-js",
		icon: faNextjs,
	},
	{
		title: "Rust",
		description: "The typed `rivetkit` crate for native Rust",
		href: "/docs/actors/quickstart/rust",
		icon: faRust,
		badge: "Beta",
	},
	{
		title: "Effect.ts",
		description: "The Effect SDK with `effect/Schema`",
		href: "/docs/actors/quickstart/effect",
		icon: faLayerGroup,
		badge: "Beta",
	},
	{
		title: "Cloudflare Workers",
		description: "Run RivetKit on Cloudflare Workers",
		href: "/docs/actors/quickstart/cloudflare",
		icon: faCloudflare,
	},
	{
		title: "Supabase Functions",
		description: "Run RivetKit on Supabase Edge Functions",
		href: "/docs/actors/quickstart/supabase",
		icon: faFunction,
	},
];

const concepts: LandingItem[] = [
	{
		title: "State & Storage",
		description: "Persist data across restarts and hibernation",
		href: "/docs/actors/state",
		icon: faFloppyDisk,
	},
	{
		title: "Actions",
		description: "The RPC surface clients call on your actor",
		href: "/docs/actors/actions",
		icon: faBolt,
	},
	{
		title: "Realtime",
		description: "Broadcast live updates to connected clients",
		href: "/docs/actors/events",
		icon: faTowerBroadcast,
	},
	{
		title: "SQLite",
		description: "An embedded SQL database per actor",
		href: "/docs/actors/sqlite",
		icon: faSqlite,
	},
];

function LandingCard({ item }: { item: LandingItem }) {
	return (
		<a
			href={item.href}
			className="group relative flex items-start gap-4 rounded-xl border border-white/10 bg-white/[0.02] p-5 no-underline transition-colors hover:border-white/25 hover:bg-white/[0.04]"
		>
			<div className="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg border border-white/10 bg-white/[0.04] text-white/70 transition-colors group-hover:text-white">
				<Icon icon={item.icon} className="text-base" />
			</div>
			<div className="flex min-w-0 flex-col gap-1">
				<div className="flex items-center gap-2">
					<span className="font-medium text-white">{item.title}</span>
					{item.badge && (
						<span className="rounded-full bg-white/10 px-2 py-0.5 text-[11px] font-medium text-white/60">
							{item.badge}
						</span>
					)}
				</div>
				<span className="text-sm leading-relaxed text-white/50">
					{item.description}
				</span>
			</div>
			<Icon
				icon={faArrowRight}
				className="absolute right-5 top-5 text-sm text-white/0 transition-all duration-200 group-hover:translate-x-0.5 group-hover:text-white/40"
			/>
		</a>
	);
}

function Section({ title, items }: { title: string; items: LandingItem[] }) {
	return (
		<section>
			<h2 className="mb-4 text-xs font-semibold uppercase tracking-wider text-white/40">
				{title}
			</h2>
			<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
				{items.map((item) => (
					<LandingCard key={item.href} item={item} />
				))}
			</div>
		</section>
	);
}

export function ActorsLanding() {
	return (
		<div className="mx-auto w-full max-w-5xl">
			<header className="mb-12">
				<h1 className="mb-2 text-2xl font-semibold tracking-tight text-white">
					Rivet Actors
				</h1>
				<p className="max-w-2xl text-base leading-relaxed text-white/50">
					Long-lived processes with durable state, realtime events, and
					built-in hibernation. Pick a stack to start building, or read the{" "}
					<a
						href="/docs/actors/crash-course"
						className="text-white/80 underline underline-offset-4 transition-colors hover:text-white"
					>
						Crash Course
					</a>{" "}
					for the core concepts.
				</p>
			</header>
			<div className="flex flex-col gap-12">
				<Section title="Get Started" items={quickstarts} />
				<Section title="Core Concepts" items={concepts} />
			</div>
		</div>
	);
}
