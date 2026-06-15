"use client";
import { getSiteUrl } from "@/lib/siteUrl";
import { Icon, faRedditAlien, faRss, faXTwitter } from "@rivet-gg/icons";
import { useState, useEffect } from "react";

export function ArticleSocials({ title }) {
	const [pathname, setPathname] = useState("");

	useEffect(() => {
		setPathname(window.location.pathname);
	}, []);

	const siteUrl = getSiteUrl();
	const articleUrl = siteUrl + pathname;
	return (
		<div className="mt-14 flex items-center justify-center gap-2">
			<SocialIcon url="/rss/feed.xml">
				<Icon icon={faRss} size="xl" />
			</SocialIcon>
			<SocialIcon
				url={`https://x.com/share?text=${encodeURIComponent(`${title} ${articleUrl} via @rivet_dev`)}`}
			>
				<Icon icon={faXTwitter} size="xl" />
			</SocialIcon>
			<SocialIcon
				url={`https://news.ycombinator.com/submitlink?u=${encodeURIComponent(
					articleUrl,
				)}&t=${encodeURIComponent(title)}`}
			>
				<HackerNewsIcon />
			</SocialIcon>
			<SocialIcon
				url={`https://www.reddit.com/submit?url=${articleUrl}&title=${encodeURIComponent(title)}`}
			>
				<Icon icon={faRedditAlien} size="xl" />
			</SocialIcon>
		</div>
	);
}

function SocialIcon({ url, children }) {
	return (
		<a
			href={url}
			target="_blank"
			rel="noreferrer"
			className="px-3 text-ink-faint transition-colors hover:text-pine"
		>
			{children}
		</a>
	);
}

// Hacker News has no boxless brand glyph in Font Awesome, so we draw the bare
// "Y" mark here to match the other boxless icons in the share row.
function HackerNewsIcon() {
	return (
		<svg
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2.7"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
			focusable="false"
			className="inline-block h-[1.45em] w-[1.45em] align-[-0.125em]"
		>
			<path d="M6 5 L12 13 L12 19.5 M18 5 L12 13" />
		</svg>
	);
}
