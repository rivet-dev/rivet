"use client";
import { cn } from "@rivet-gg/components";
import { Icon, faGithub } from "@rivet-gg/icons";
import { useEffect, useState } from "react";

type GitHubDropdownProps = React.HTMLAttributes<HTMLAnchorElement>;

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

const REPOS = [
	"rivet-dev/rivet",
	"rivet-dev/secure-exec",
	"rivet-dev/sandbox-agent",
	"rivet-dev/antiox",
	"rivet-dev/agent-os",
];

export function GitHubDropdown({ className, ...props }: GitHubDropdownProps) {
	const [totalStars, setTotalStars] = useState<RepoData>({
		stars: 0,
		loading: true,
	});

	useEffect(() => {
		const cacheKey = "github-stars-aggregate";
		const cachedData = sessionStorage.getItem(cacheKey);

		if (cachedData) {
			const { stars: cachedStars, timestamp } = JSON.parse(cachedData);
			if (Date.now() - timestamp < 5 * 60 * 1000) {
				setTotalStars({ stars: cachedStars, loading: false });
				return;
			}
		}

		Promise.allSettled(
			REPOS.map((repo) =>
				fetch(`https://api.github.com/repos/${repo}`)
					.then((res) => (res.ok ? res.json() : null))
					.then((data) => data?.stargazers_count ?? 0)
					.catch(() => 0),
			),
		).then((results) => {
			const total = results.reduce(
				(sum, r) => sum + (r.status === "fulfilled" ? r.value : 0),
				0,
			);
			setTotalStars({ stars: total, loading: false });
			sessionStorage.setItem(
				cacheKey,
				JSON.stringify({ stars: total, timestamp: Date.now() }),
			);
		});
	}, []);

	return (
		<a
			href="https://github.com/rivet-dev"
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
				{totalStars.loading ? "GitHub" : `${formatNumber(totalStars.stars)} stars`}
			</span>
		</a>
	);
}
