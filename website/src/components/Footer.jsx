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
		{ name: "Sandbox Agent SDK", href: "https://sandboxagent.dev" },
		{ name: "Pricing", href: "/cloud#pricing" },
		{ name: "Talk to an engineer", href: "/talk-to-an-engineer" },
		{ name: "Sales", href: "/sales" },
	],
	devs: [
		{ name: "Documentation", href: "/docs/actors" },
		// { name: "Integrations", href: "/integrations" },
		// { name: "API Reference", href: "/docs/api" },
		{ name: "Changelog", href: "/changelog" },
		{ name: "Status Page", href: "https://rivet.betteruptime.com/" },
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
		<div className="mx-auto max-w-7xl w-full py-16 selection:bg-[#FF4500]/30 selection:text-orange-200">
			<div className="grid grid-cols-2 gap-8 md:grid-cols-4 lg:grid-cols-5">
				{/* Brand column */}
				<div className="col-span-2 md:col-span-4 lg:col-span-1 space-y-6">
					<img className="h-8 w-8" src={imgLogo.src} alt="Rivet" />
					<p className="text-sm text-zinc-500">
						Infrastructure for software that thinks
					</p>
					<div className="flex gap-4">
						{footer.social.map((item) => (
							<a
								key={item.name}
								href={item.href}
								className="text-zinc-600 hover:text-white transition-colors"
							>
								<span className="sr-only">{item.name}</span>
								<Icon icon={item.icon} aria-hidden="true" />
							</a>
						))}
					</div>
				</div>

				{/* Product */}
				<div>
					<h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-4">Product</h3>
					<ul className="space-y-3">
						{footer.product.map((item) => (
							<li key={item.name}>
								<a
									href={item.href}
									target={item.target}
									className="text-sm text-zinc-400 hover:text-white transition-colors"
								>
									{item.name}
								</a>
							</li>
						))}
					</ul>
				</div>

				{/* Developers */}
				<div>
					<h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-4">Developers</h3>
					<ul className="space-y-3">
						{footer.devs.map((item) => (
							<li key={item.name}>
								<a
									href={item.href}
									target={item.target}
									className="text-sm text-zinc-400 hover:text-white transition-colors"
								>
									{item.name}
								</a>
							</li>
						))}
					</ul>
				</div>

				{/* Resources */}
				<div>
					<h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-4">Resources</h3>
					<ul className="space-y-3">
						{footer.resources.map((item) => (
							<li key={item.name}>
								<a
									href={item.href}
									target={item.newTab ? "_blank" : null}
									className="text-sm text-zinc-400 hover:text-white transition-colors"
								>
									{item.name}
								</a>
							</li>
						))}
					</ul>
				</div>

				{/* Legal */}
				<div>
					<h3 className="text-xs font-medium uppercase tracking-wider text-zinc-500 mb-4">Legal</h3>
					<ul className="space-y-3">
						{footer.legal.map((item) => (
							<li key={item.name}>
								<a
									href={item.href}
									className="text-sm text-zinc-400 hover:text-white transition-colors"
								>
									{item.name}
								</a>
							</li>
						))}
					</ul>
				</div>
			</div>

			{/* Investor badges */}
			<div className="mt-12 flex flex-wrap items-center gap-4">
				<span className="text-xs text-zinc-600">Backed by</span>
				<div className="flex flex-wrap items-center gap-2">
					<div className="flex items-center gap-2 rounded-full border border-white/10 px-3 py-1.5 text-xs text-zinc-400">
						<img src={imgYC.src} alt="Y Combinator" className="h-4 w-auto" />
						<span>Y Combinator</span>
					</div>
					<div className="flex items-center gap-2 rounded-full border border-white/10 px-3 py-1.5 text-xs text-zinc-400">
						<img src={imgA16z.src} alt="a16z" className="h-3 w-auto" />
						<span>a16z Speedrun</span>
					</div>
				</div>
				<a
					href="/startups"
					className="text-xs text-zinc-500 hover:text-white transition-colors"
					style={{
						backgroundImage: 'radial-gradient(circle, currentColor 1px, transparent 1px)',
						backgroundSize: '6px 1px',
						backgroundPosition: '0 100%',
						backgroundRepeat: 'repeat-x',
						paddingBottom: '4px'
					}}
				>
					Are you as well?
				</a>
				<span className="ml-auto flex items-center gap-1.5 text-xs text-zinc-600">
					<svg width="14" height="10" viewBox="0 0 14 10" fill="none">
						<rect width="14" height="10" fill="white" />
						<rect y="0" width="14" height="1.2" fill="black" />
						<rect y="2.2" width="14" height="1.2" fill="black" />
						<rect y="4.4" width="14" height="1.2" fill="black" />
						<rect y="6.6" width="14" height="1.2" fill="black" />
						<rect y="8.8" width="14" height="1.2" fill="black" />
						<rect width="5" height="5" fill="black" />
					</svg>
					Built in San Francisco, United States
				</span>
			</div>

			{/* Copyright */}
			<div className="mt-12 border-t border-white/10 pt-8">
				<p className="text-xs text-zinc-600">
					&copy; {new Date().getFullYear()} Rivet Gaming, Inc. All rights reserved.
				</p>
				<p className="mt-2 text-xs text-zinc-700">
					Cloudflare® and Durable Objects™ are trademarks of Cloudflare, Inc. No affiliation or endorsement implied.
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
