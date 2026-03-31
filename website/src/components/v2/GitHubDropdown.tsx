"use client";
import { cn } from "@rivet-gg/components";
import { Icon, faGithub } from "@rivet-gg/icons";
import { useEffect, useState } from "react";

interface GitHubDropdownProps extends React.HTMLAttributes<HTMLDivElement> {}

interface RepoData {
	stars: number;
	loading: boolean;
}

function formatNumber(num: number): string {
	if (num >= 1000) {
		return `${(num / 1000).toFixed(1)}k`;
	}
	return num.toString();
}

export function GitHubDropdown({ className, ...props }: GitHubDropdownProps) {
	const [rivetStars, setRivetStars] = useState<RepoData>({
		stars: 0,
		loading: true,
	});

	const fetchStars = async (
		repo: string,
		setter: (data: RepoData) => void,
	) => {
		const cacheKey = `github-stars-${repo}`;
		const cachedData = sessionStorage.getItem(cacheKey);

		if (cachedData) {
			const { stars: cachedStars, timestamp } = JSON.parse(cachedData);
			if (Date.now() - timestamp < 5 * 60 * 1000) {
				setter({ stars: cachedStars, loading: false });
				return;
			}
		}

		try {
			const response = await fetch(
				`https://api.github.com/repos/${repo}`,
			);
			if (!response.ok) throw new Error("Failed to fetch");
			const data = await response.json();
			const newStars = data.stargazers_count;
			setter({ stars: newStars, loading: false });
			sessionStorage.setItem(
				cacheKey,
				JSON.stringify({
					stars: newStars,
					timestamp: Date.now(),
				}),
			);
		} catch (err) {
			console.error(`Failed to fetch stars for ${repo}`, err);
			setter({ stars: 0, loading: false });
		}
	};

	useEffect(() => {
		fetchStars("rivet-dev/rivet", setRivetStars);
	}, []);

	return (
		<a
			href="https://github.com/rivet-dev/rivet"
			target="_blank"
			rel="noreferrer"
			className={cn(
				"flex items-center gap-2 transition-colors",
				className?.includes("text-white")
					? "text-white/90 hover:text-white"
					: "text-muted-foreground hover:text-foreground",
				className,
			)}
			{...props}
		>
			<Icon icon={faGithub} />
			<span className="hidden md:inline">
				{rivetStars.loading ? "GitHub" : `${formatNumber(rivetStars.stars)} stars`}
			</span>
		</a>
	);
}
