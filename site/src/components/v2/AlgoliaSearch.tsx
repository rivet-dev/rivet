"use client";

import algoliasearch from "algoliasearch";
import { Button, Dialog, DialogPortal, Kbd, cn } from "@rivet-gg/components";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

const searchClient = algoliasearch(
	process.env.NEXT_PUBLIC_ALGOLIA_APP_ID || "YourAppId",
	process.env.NEXT_PUBLIC_ALGOLIA_SEARCH_API_KEY || "YourSearchKey"
);

interface SearchResult {
	objectID: string;
	title: string;
	content: string;
	url: string;
	hierarchy: {
		lvl0?: string;
		lvl1?: string;
		lvl2?: string;
	};
}

export function AlgoliaSearch() {
	const [isOpen, setIsOpen] = useState(false);
	const [query, setQuery] = useState("");
	const [results, setResults] = useState<SearchResult[]>([]);
	const [isLoading, setIsLoading] = useState(false);
	const router = useRouter();

	useEffect(function setShortcutListener() {
		const handleKeyDown = (e: KeyboardEvent) => {
			if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
				e.preventDefault();
				setIsOpen((prev) => !prev);
			}
		};

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, []);

	useEffect(() => {
		if (!query.trim()) {
			setResults([]);
			return;
		}

		const searchDebounce = setTimeout(async () => {
			setIsLoading(true);
			try {
				const { hits } = await searchClient
					.initIndex(process.env.NEXT_PUBLIC_ALGOLIA_INDEX_NAME || "rivet-docs")
					.search(query, {
						hitsPerPage: 8,
						attributesToRetrieve: ["title", "content", "url", "hierarchy"],
						attributesToHighlight: ["title", "content"],
					});

				setResults(hits as SearchResult[]);
			} catch (error) {
				console.error("Search error:", error);
			} finally {
				setIsLoading(false);
			}
		}, 300);

		return () => clearTimeout(searchDebounce);
	}, [query]);

	const handleResultClick = (result: SearchResult) => {
		router.push(result.url);
		setIsOpen(false);
		setQuery("");
	};

	return (
		<>
			<Button
				onClick={() => setIsOpen(true)}
				variant="outline"
				className={cn(
					"relative h-8 w-full justify-start rounded-[0.5rem] bg-background text-sm font-normal text-muted-foreground shadow-none hidden md:flex md:w-24 lg:w-40",
				)}
			>
				<span className="hidden lg:inline-flex">Search...</span>
				<span className="inline-flex lg:hidden">Search...</span>
				<Kbd className="absolute right-[0.3rem] top-1/2 -translate-y-1/2 hidden sm:flex">
					<Kbd.Key />K
				</Kbd>
			</Button>
			<Dialog open={isOpen}>
				<DialogPortal>
					<div className="fixed inset-0 z-50 bg-background/80 backdrop-blur-sm" onClick={() => setIsOpen(false)}>
						<div 
							className="fixed left-[50%] top-[50%] z-50 w-full max-w-lg translate-x-[-50%] translate-y-[-50%] rounded-lg border bg-background shadow-lg"
							onClick={(e) => e.stopPropagation()}
						>
							<div className="flex items-center border-b px-3">
								<svg className="mr-2 h-4 w-4 shrink-0 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
									<circle cx="11" cy="11" r="8" />
									<path d="M21 21l-4.35-4.35" />
								</svg>
								<input
									value={query}
									onChange={(e) => setQuery(e.target.value)}
									className="flex h-11 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground"
									placeholder="Search documentation..."
									autoFocus
								/>
							</div>
							<div className="max-h-[300px] overflow-y-auto">
								{isLoading && (
									<div className="p-4 text-center text-sm text-muted-foreground">
										Searching...
									</div>
								)}
								{!isLoading && query && results.length === 0 && (
									<div className="p-4 text-center text-sm text-muted-foreground">
										No results found for "{query}"
									</div>
								)}
								{!isLoading && results.map((result) => (
									<div
										key={result.objectID}
										className="p-3 hover:bg-muted cursor-pointer border-b border-border last:border-b-0"
										onClick={() => handleResultClick(result)}
									>
										<div className="font-medium text-sm">{result.title}</div>
										{result.hierarchy?.lvl1 && (
											<div className="text-xs text-primary mb-1">{result.hierarchy.lvl1}</div>
										)}
										<div className="text-xs text-muted-foreground line-clamp-2">
											{result.content?.substring(0, 150)}...
										</div>
									</div>
								))}
							</div>
							<div className="flex items-center justify-between p-3 pt-2 border-t">
								<p className="text-xs text-muted-foreground">
									Press{" "}
									<kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium">
										⌘K
									</kbd>{" "}
									to search
								</p>
								<Button
									variant="ghost"
									size="sm"
									onClick={() => setIsOpen(false)}
									className="h-auto p-1 text-xs"
								>
									ESC
								</Button>
							</div>
						</div>
					</div>
				</DialogPortal>
			</Dialog>
		</>
	);
}