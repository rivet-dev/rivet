"use client";
import { getSiteUrl } from "@/lib/siteUrl";
import {
	Icon,
	faHackerNews,
	faReddit,
	faRssSquare,
	faXTwitter,
} from "@rivet-gg/icons";
import { useState, useEffect } from "react";

export function ArticleSocials({ title }) {
	const [pathname, setPathname] = useState("");

	useEffect(() => {
		setPathname(window.location.pathname);
	}, []);

	const siteUrl = getSiteUrl();
	const articleUrl = siteUrl + pathname;
	return (
		<div className="mt-14 flex items-center justify-center">
			<SocialIcon url="/rss/feed.xml" icon={faRssSquare} />
			<SocialIcon
				url={`https://x.com/share?text=${encodeURIComponent(`${title} ${articleUrl} via @rivet_dev`)}`}
				icon={faXTwitter}
			/>
			<SocialIcon
				url={`https://news.ycombinator.com/submitlink?u=${encodeURIComponent(
					articleUrl,
				)}&t=${encodeURIComponent(title)}`}
				icon={faHackerNews}
			/>
			<SocialIcon
				url={`https://www.reddit.com/submit?url=${articleUrl}&title=${encodeURIComponent(title)}`}
				icon={faReddit}
			/>
		</div>
	);
}

function SocialIcon({ url, icon }) {
	return (
		<a
			href={url}
			target="_blank"
			rel="noreferrer"
			className="px-3 text-ink-faint transition-colors hover:text-pine"
		>
			<Icon icon={icon} size="xl" />
		</a>
	);
}
