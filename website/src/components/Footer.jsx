"use client";
import { usePathname } from "@/hooks/usePathname";
import { Button } from "@/components/Button";
import routes from "@/generated/routes.json";
import clsx from "clsx";

import imgLogo from "@/images/rivet-logos/icon-white.svg";
import imgYC from "@/images/logos/yc.svg";
import imgA16z from "@/images/logos/a16z.svg";
import {
	Icon,
	faBluesky,
	faDiscord,
	faGithub,
	faLinkedin,
	faTwitter,
	faYoutube,
} from "@rivet-gg/icons";

const footer = {
	product: [
		{ name: "Actors", href: "/docs/actors" },
		{ name: "Pricing", href: "/cloud#pricing" },
		{ name: "Talk to an engineer", href: "/talk-to-an-engineer" },
		{ name: "Sales", href: "/sales" },
	],
	solutions: [
		{ name: "Agent Orchestration", href: "/solutions/agents" },
		{ name: "Multiplayer Documents", href: "/solutions/collaborative-state" },
		{ name: "Workflows", href: "/solutions/workflows" },
		{ name: "Vibe-Coded Backends", href: "/solutions/app-generators" },
		{ name: "Geo-Distributed Databases", href: "/solutions/geo-distributed-db" },
		{ name: "Per-Tenant Databases", href: "/solutions/per-tenant-db" },
	],
	devs: [
		{ name: "Documentation", href: "/docs/actors" },
		// { name: "Integrations", href: "/integrations" },
		// { name: "API Reference", href: "/docs/api" },
		{ name: "Changelog", href: "/changelog" },
		// NOTE: Disabled until v2
		// { name: "Status Page", href: "https://rivet.betteruptime.com/" },
	],
	resources: [
		{ name: "Blog", href: "/blog" },
		{
			name: "Rivet vs Cloudflare Workers",
			href: "/rivet-vs-cloudflare-workers",
		},
		{ name: "YC & Speedrun Deal", href: "/startups" },
		{ name: "Open-Source Friends", href: "/oss-friends" },
		{ name: "Press Kit", href: "https://releases.rivet.dev/press-kit.zip" },
	],
	legal: [
		{ name: "Terms", href: "/terms" },
		{ name: "Privacy Policy", href: "/privacy" },
		{ name: "Acceptable Use", href: "/acceptable-use" },
	],
	social: [
		{
			name: "Discord",
			href: "https://discord.gg/aXYfyNxYVn",
			icon: faDiscord,
		},
		{
			name: "Twitter",
			href: "https://x.com/rivet_dev",
			icon: faTwitter,
		},
		{
			name: "Bluesky",
			href: "https://bsky.app/profile/rivet.dev",
			icon: faBluesky,
		},
		{
			name: "GitHub",
			href: "https://github.com/rivet-dev",
			icon: faGithub,
		},
		{
			name: "YouTube",
			href: "https://www.youtube.com/@rivet-dev",
			icon: faYoutube,
		},
		{
			name: "LinkedIn",
			href: "https://www.linkedin.com/company/72072261/",
			icon: faLinkedin,
		},
	],
};

function PageLink({ label, page, previous = false }) {
	const title = routes.pages[page.href]?.title ?? page.title ?? label;
	return (
		<>
			<Button
				href={page.href}
				aria-label={`${label}: ${page.title}`}
				variant="secondary"
				arrow={previous ? "left" : "right"}
			>
				{title}
			</Button>
		</>
	);
}

export function PageNextPrevious({ navigation }) {
	const pathname = usePathname();
	const allPages = navigation.sidebar.groups.flatMap((group) => group.pages);
	const currentPageIndex = allPages.findIndex(
		(page) => page.href === pathname,
	);

	if (currentPageIndex === -1) {
		return null;
	}

	const previousPage = allPages[currentPageIndex - 1];
	const nextPage = allPages[currentPageIndex + 1];

	if (!previousPage && !nextPage) {
		return null;
	}

	return (
		<div className={clsx("mb-4 flex", "mx-auto max-w-5xl")}>
			{previousPage && (
				<div className="flex flex-col items-start gap-3">
					<PageLink label="Previous" page={previousPage} previous />
				</div>
			)}
			{nextPage && (
				<div className="ml-auto flex flex-col items-end gap-3">
					<PageLink label="Next" page={nextPage} />
				</div>
			)}
		</div>
	);
}

function SmallPrint() {
	return (
		<div className="mx-auto max-w-screen-2xl w-full pb-8 pt-16 sm:pt-20">
			<div className="xl:grid xl:grid-cols-12 xl:gap-24">
				{/* Brands & links */}
				<div className="space-y-8 xl:col-span-3">
					{/* Logo */}
					<img className="size-12" src={imgLogo.src} alt="Rivet" />
					<p className="text-sm leading-6 text-gray-300">
						Build and scale stateful workloads
					</p>

					{/* Social */}
					<div className="flex space-x-6">
						{footer.social.map((item) => (
							<a
								key={item.name}
								href={item.href}
								className="text-xl text-gray-500 hover:text-gray-400"
							>
								<span className="sr-only">{item.name}</span>
								<Icon icon={item.icon} aria-hidden="true" />
							</a>
						))}
					</div>

					{/* Investor badges */}
					<div>
						<p className="mb-2 text-xs text-white">Backed by</p>
						<div className="flex flex-wrap sm:flex-nowrap items-center gap-3">
							<div className="flex shrink-0 items-center gap-2 whitespace-nowrap rounded-full border border-white/20 bg-white/5 px-3 py-1.5 text-xs text-white/70">
								<img src={imgYC.src} alt="Y Combinator" className="h-4 w-auto" />
								<span>Y Combinator</span>
							</div>
							<div className="flex shrink-0 items-center gap-2 whitespace-nowrap rounded-full border border-white/20 bg-white/5 px-3 py-1.5 text-xs text-white/70">
								<img src={imgA16z.src} alt="a16z" className="h-3 w-auto" />
								<span>a16z Speedrun</span>
							</div>
						</div>
					</div>
				</div>

				{/* Links */}
				<div className="mt-16 grid grid-cols-1 gap-x-12 gap-y-8 md:grid-cols-2 lg:grid-cols-5 xl:col-span-9 xl:mt-0">
					<div>
						<div className="text-sm font-semibold leading-6 text-white">
							Product
						</div>
						<ul role="list" className="mt-3 space-y-2">
							{footer.product.map((item) => (
								<li key={item.name}>
									<a href={item.href}
										target={item.target}
										className="text-sm leading-4 text-gray-300 hover:text-white"
									>
										{item.name}
									</a>
								</li>
							))}
						</ul>
					</div>
					<div>
						<div className="text-sm font-semibold leading-6 text-white">
							Solutions
						</div>
						<ul role="list" className="mt-3 space-y-2">
							{footer.solutions.map((item) => (
								<li key={item.name}>
									<a
										href={item.href}
										target={item.target}
										className="text-sm leading-4 text-gray-300 hover:text-white"
									>
										{item.name}
									</a>
								</li>
							))}
						</ul>
					</div>
					<div>
						<div className="text-sm font-semibold leading-6 text-white">
							Developers
						</div>
						<ul role="list" className="mt-3 space-y-2">
							{footer.devs.map((item) => (
								<li key={item.name}>
									<a href={item.href}
										target={item.target}
										className="text-sm leading-4 text-gray-300 hover:text-white"
									>
										{item.name}
									</a>
								</li>
							))}
						</ul>
					</div>
					<div>
						<div className="text-sm font-semibold leading-6 text-white">
							Resources
						</div>
						<ul role="list" className="mt-3 space-y-2">
							{footer.resources.map((item) => (
								<li key={item.name}>
									<a href={item.href}
										target={item.newTab ? "_blank" : null}
										className={clsx(
											"text-sm leading-4 text-gray-300 hover:text-white",
										)}
									>
										<span
											className={clsx(
												item.highlight &&
													"text-violet-200 drop-shadow-[0_0_10px_rgba(221,214,254,0.5)]",
											)}
										>
											{item.name}
										</span>
										{item.badge && (
											<span className="ml-2 rounded-full bg-violet-500 px-2">
												{item.badge}
											</span>
										)}
									</a>
								</li>
							))}
						</ul>
					</div>
					<div>
						<div className="text-sm font-semibold leading-6 text-white">
							Legal
						</div>
						<ul role="list" className="mt-3 space-y-2">
							{footer.legal.map((item) => (
								<li key={item.name}>
									<a href={item.href}
										className="text-sm leading-4 text-gray-300 hover:text-white"
									>
										{item.name}
									</a>
								</li>
							))}
						</ul>
					</div>
				</div>
			</div>

			{/* Footer */}
			<div className="mt-4 border-t border-white/10 pt-4 text-center md:mt-8">
				<p className="text-xs leading-5 text-white">
					&copy; {new Date().getFullYear()} Rivet Gaming, Inc. All
					rights reserved.
				</p>
				<p className="text-xs leading-5 text-white/30">
					Cloudflare® and Durable Objects™ are trademarks of
					Cloudflare, Inc. No affiliation or endorsement implied.
					References used for comparison purposes only.
				</p>
			</div>
		</div>
	);
}

export function Footer() {
	return (
		<div>
			<hr className="mb-8 border-white/10" />

			<footer
				aria-labelledby="footer-heading"
				className="mx-auto max-w-screen-2xl px-6 lg:px-12"
			>
				<h2 id="footer-heading" className="sr-only">
					Footer
				</h2>
				<SmallPrint />
			</footer>
		</div>
	);
}
