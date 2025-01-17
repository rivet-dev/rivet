"use client";
import { getSiteUrl } from "@/lib/siteUrl";
import {
	Icon,
	faHackerNews,
	faReddit,
	faRssSquare,
	faXTwitter,
} from "@rivet-gg/icons";
import { usePathname } from "next/navigation";

export function ArticleSocials({ title }) {
	const pathname = usePathname();
	const siteUrl = getSiteUrl();
	const articleUrl = siteUrl + pathname;
	return (
		<div className="after:bg-secondary relative mt-14 flex items-center justify-center after:absolute after:inset-x-0 after:-z-[1] after:h-[1px]">
			<SocialIcon url="/rss/feed.xml" icon={faRssSquare} />
			<SocialIcon
				url={`https://x.com/share?text=${encodeURIComponent(`${title} ${articleUrl} via @rivet_gg`)}`}
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
			className="text-primary hover:text-primary/80 bg-background px-3 transition-colors"
		>
			<Icon icon={icon} size="xl" />
		</a>
	);
}
