"use client";
import { cn } from "@rivet-gg/components";
import { Icon, faGithub } from "@rivet-gg/icons";
import { useEffect, useState } from "react";

interface GitHubStarsProps
	extends React.AnchorHTMLAttributes<HTMLAnchorElement> {
	repo?: string;
}

function formatNumber(num: number): string {
	if (num >= 1000) {
		return `${(num / 1000).toFixed(1)}k`;
	}
	return num.toString();
}

export function GitHubStars({
	repo = "rivet-gg/rivet",
	className,
	...props
}: GitHubStarsProps) {
	const [stars, setStars] = useState<number | null>(null);

	useEffect(() => {
		const cacheKey = `github-stars-${repo}`;
		const cachedData = sessionStorage.getItem(cacheKey);

		if (cachedData) {
			const { stars: cachedStars, timestamp } = JSON.parse(cachedData);
			// Check if cache is less than 5 minutes old
			if (Date.now() - timestamp < 5 * 60 * 1000) {
				setStars(cachedStars);
				return;
			}
		}

		fetch(`https://api.github.com/repos/${repo}`)
			.then((response) => {
				if (!response.ok) throw new Error("Failed to fetch");
				return response.json();
			})
			.then((data) => {
				const newStars = data.stargazers_count;
				setStars(newStars);
				sessionStorage.setItem(
					cacheKey,
					JSON.stringify({
						stars: newStars,
						timestamp: Date.now(),
					}),
				);
			})
			.catch((err) => {
				console.error("Failed to fetch stars", err);
			});
	}, [repo]);

	return (
		<a
			href={`https://github.com/${repo}`}
			target="_blank"
			rel="noreferrer"
			className={cn(
				"md:bg-white/10 rounded-md px-4 h-10 flex items-center gap-2 md:hover:bg-white/20 transition-colors",
				className,
			)}
			{...props}
		>
			<Icon icon={faGithub} />
			<span className="hidden md:inline">
				{stars ? `${formatNumber(stars)} Stars` : "GitHub"}
			</span>
		</a>
	);
}
