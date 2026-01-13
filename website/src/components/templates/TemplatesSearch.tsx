"use client";

import { MagnifyingGlassIcon } from "@heroicons/react/24/outline";
import { useTemplatesFilter } from "./TemplatesFilterContext";

export function TemplatesSearch() {
	const { searchQuery, setSearchQuery } = useTemplatesFilter();

	return (
		<div className="relative">
			<div className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3">
				<MagnifyingGlassIcon
					className="h-5 w-5 text-zinc-400"
					aria-hidden="true"
				/>
			</div>
			<input
				type="text"
				value={searchQuery}
				onChange={(e) => setSearchQuery(e.target.value)}
				className="block w-full rounded-lg border border-white/20 bg-white/5 pl-10 pr-3 py-3 text-white placeholder:text-zinc-500 focus:border-white/50 focus:outline-none focus:ring-1 focus:ring-white/50 text-base"
				placeholder="Search templates..."
			/>
		</div>
	);
}
