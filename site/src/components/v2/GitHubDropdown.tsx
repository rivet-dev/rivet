"use client";
import { cn } from "@rivet-gg/components";
import { Icon, faArrowRight, faGithub, faRust, faNodeJs, faTs } from "@rivet-gg/icons";
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
	const [rivetKitStars, setRivetKitStars] = useState<RepoData>({
		stars: 0,
		loading: true,
	});
	const [isOpen, setIsOpen] = useState(false);

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
		fetchStars("rivet-gg/rivet", setRivetStars);
		fetchStars("rivet-gg/rivetkit", setRivetKitStars);
	}, []);

	const totalStars = rivetStars.stars + rivetKitStars.stars;
	const isLoading = rivetStars.loading || rivetKitStars.loading;

	return (
		<div
			className={cn("relative", className)}
			onMouseEnter={() => setIsOpen(true)}
			onMouseLeave={() => setIsOpen(false)}
			{...props}
		>
			<button
				className={cn(
					"flex items-center gap-2 transition-colors",
					className?.includes("text-white")
						? "text-white/90 hover:text-white"
						: "text-muted-foreground hover:text-foreground",
				)}
				aria-expanded={isOpen}
				aria-haspopup="true"
			>
				<Icon icon={faGithub} />
				<span className="hidden md:inline">
					{isLoading ? "GitHub" : `${formatNumber(totalStars)} stars`}
				</span>
			</button>

			{isOpen && (
				<div className="absolute right-0 top-full pt-1 w-72 z-50">
					<div
						className={cn(
							"rounded-md border shadow-lg",
							className?.includes("text-white")
								? "border-white/10 bg-background/95 backdrop-blur-sm"
								: "border-border bg-card",
						)}
					>
						<div className="py-1">
							<a
								href="https://github.com/rivet-gg/rivet"
								target="_blank"
								rel="noreferrer"
								className={cn(
									"group flex items-center justify-between px-4 py-3 text-sm transition-colors",
									className?.includes("text-white")
										? "text-white/90 hover:bg-white/5 hover:text-white"
										: "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
								)}
							>
								<div className="flex items-center gap-3">
									<Icon
										icon={faRust}
										className={cn(
											"h-4 w-4",
											className?.includes("text-white")
												? "text-white/70"
												: "text-muted-foreground",
										)}
									/>
									<div className="flex flex-col items-start">
										<span className="font-medium">Rivet</span>
										<span
											className={cn(
												"text-xs",
												className?.includes("text-white")
													? "text-white/60"
													: "text-muted-foreground",
											)}
										>
											Stateful workload orchestrator
										</span>
										<span
											className={cn(
												"text-xs mt-0.5",
												className?.includes("text-white")
													? "text-white/70"
													: "text-muted-foreground",
											)}
										>
											{rivetStars.loading
												? "..."
												: `${formatNumber(rivetStars.stars)} stars`}
										</span>
									</div>
								</div>
								<Icon
									icon={faArrowRight}
									className="h-3 w-3 opacity-0 group-hover:opacity-50 transition-opacity"
								/>
							</a>
							<a
								href="https://github.com/rivet-gg/rivetkit"
								target="_blank"
								rel="noreferrer"
								className={cn(
									"group flex items-center justify-between px-4 py-3 text-sm transition-colors",
									className?.includes("text-white")
										? "text-white/90 hover:bg-white/5 hover:text-white"
										: "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
								)}
							>
								<div className="flex items-center gap-3">
									<Icon
										icon={faTs}
										className={cn(
											"h-4 w-4",
											className?.includes("text-white")
												? "text-white/70"
												: "text-muted-foreground",
										)}
									/>
									<div className="flex flex-col items-start">
										<span className="font-medium">RivetKit</span>
										<span
											className={cn(
												"text-xs",
												className?.includes("text-white")
													? "text-white/60"
													: "text-muted-foreground",
											)}
										>
											Library for building stateful workloads
										</span>
										<span
											className={cn(
												"text-xs mt-0.5",
												className?.includes("text-white")
													? "text-white/70"
													: "text-muted-foreground",
											)}
										>
											{rivetKitStars.loading
												? "..."
												: `${formatNumber(rivetKitStars.stars)} stars`}
										</span>
									</div>
								</div>
								<Icon
									icon={faArrowRight}
									className="h-3 w-3 opacity-0 group-hover:opacity-50 transition-opacity"
								/>
							</a>
						</div>
					</div>
				</div>
			)}
		</div>
	);
}
