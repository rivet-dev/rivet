"use client";

import Typesense from "typesense";
import { Button, Dialog, DialogPortal, Kbd, cn } from "@rivet-gg/components";
import { useEffect, useState } from "react";

const searchClient = new Typesense.Client({
	nodes: [
		{
			host: import.meta.env.PUBLIC_TYPESENSE_HOST || "localhost",
			port: Number(import.meta.env.PUBLIC_TYPESENSE_PORT) || 443,
			protocol: import.meta.env.PUBLIC_TYPESENSE_PROTOCOL || "https",
		},
	],
	apiKey: import.meta.env.PUBLIC_TYPESENSE_SEARCH_API_KEY || "xyz",
	connectionTimeoutSeconds: 2,
});

interface SearchResult {
	id: string;
	title: string;
	content: string;
	url: string;
	hierarchy: {
		lvl0?: string;
		lvl1?: string;
		lvl2?: string;
	};
}

export function TypesenseSearch() {
	const [isOpen, setIsOpen] = useState(false);
	const [query, setQuery] = useState("");
	const [results, setResults] = useState<SearchResult[]>([]);
	const [isLoading, setIsLoading] = useState(false);
	const [selectedIndex, setSelectedIndex] = useState(0);
	const [inputFocused, setInputFocused] = useState(false);

	const hasRequiredKeys = !!(
		import.meta.env.PUBLIC_TYPESENSE_HOST &&
		import.meta.env.PUBLIC_TYPESENSE_SEARCH_API_KEY
	);

	if (!hasRequiredKeys) {
		return null;
	}

	const handleResultClick = (result: SearchResult) => {
		window.location.href = result.url;
		setIsOpen(false);
		setQuery("");
	};

	useEffect(function setShortcutListener() {
		const handleKeyDown = (e: KeyboardEvent) => {
			if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
				e.preventDefault();
				setIsOpen((prev) => !prev);
			}
			if (e.key === "Escape") {
				if (isOpen) {
					if (!inputFocused) {
						e.preventDefault();
						e.stopPropagation();
						setIsOpen(false);
					}
				}
			}
			if (isOpen && results.length > 0) {
				if (e.key === "ArrowDown") {
					e.preventDefault();
					setSelectedIndex((prev) => (prev + 1) % results.length);
				}
				if (e.key === "ArrowUp") {
					e.preventDefault();
					setSelectedIndex((prev) => (prev - 1 + results.length) % results.length);
				}
				if (e.key === "Enter" && selectedIndex >= 0) {
					e.preventDefault();
					handleResultClick(results[selectedIndex]);
				}
			}
		};

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [isOpen, results, selectedIndex, handleResultClick]);

	useEffect(() => {
		if (!query.trim()) {
			setResults([]);
			setIsLoading(false);
			return;
		}

		// Show loading immediately when typing
		setIsLoading(true);

		const searchDebounce = setTimeout(async () => {
			try {
				const searchResults = await searchClient
					.collections(
						import.meta.env.PUBLIC_TYPESENSE_COLLECTION_NAME ||
						"rivet-docs",
					)
					.documents()
					.search({
						q: query,
						query_by: "title,content",
						per_page: 8,
						highlight_full_fields: "title,content",
					});

				const hits =
					searchResults.hits?.map((hit) => ({
						id: hit.document.id,
						title: hit.document.title,
						content: hit.document.content,
						url: hit.document.url,
						hierarchy: hit.document.hierarchy || {},
					})) || [];

				setResults(hits);
				setSelectedIndex(0); // Reset selection to first result
			} catch (error) {
				console.error("Search error:", error);
			} finally {
				setIsLoading(false);
			}
		}, 300);

		return () => clearTimeout(searchDebounce);
	}, [query]);

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
					<div
						className="fixed inset-0 z-50 bg-background/80 backdrop-blur-sm"
						onClick={() => setIsOpen(false)}
					>
						<div
							className="fixed left-[50%] top-[50%] z-50 w-full max-w-lg translate-x-[-50%] translate-y-[-50%] rounded-lg border bg-background shadow-lg"
							onClick={(e) => e.stopPropagation()}
						>
							<div className="flex items-center border-b px-3">
								<input
									value={query}
									onChange={(e) => setQuery(e.target.value)}
									onFocus={() => setInputFocused(true)}
									onBlur={() => setInputFocused(false)}
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
								{!isLoading &&
									query &&
									results.length === 0 && (
										<div className="p-4 text-center text-sm text-muted-foreground">
											No results found for "{query}"
										</div>
									)}
								{!isLoading &&
									results.map((result, index) => (
										<div
											key={result.id}
											className={cn(
												"p-3 hover:bg-muted cursor-pointer border-b border-border last:border-b-0",
												index === selectedIndex && "bg-muted"
											)}
											onClick={() =>
												handleResultClick(result)
											}
										>
											<div className="font-medium text-sm">
												{result.title}
											</div>
											{result.hierarchy?.lvl1 && (
												<div className="text-xs text-primary mb-1">
													{result.hierarchy.lvl1}
												</div>
											)}
											<div className="text-xs text-muted-foreground line-clamp-2">
												{result.content?.substring(
													0,
													150,
												)}
												...
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
