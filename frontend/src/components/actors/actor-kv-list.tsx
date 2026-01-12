import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { type PropsWithChildren, useEffect, useRef } from "react";
import { useActor } from "./actor-queries-context";
import { ActorObjectInspector } from "./console/actor-inspector";
import type { ActorId } from "./queries";

interface ActorKvListProps {
	actorId: ActorId;
	search: string;
}

export function ActorKvList({
	actorId,
	search,
}: ActorKvListProps) {
	const actorQueries = useActor();
	const { data, isLoading, isError } = useQuery(
		actorQueries.actorKvQueryOptions(actorId),
	);

	if (isLoading) {
		return <Info>Loading KV entries...</Info>;
	}

	if (isError) {
		return (
			<Info>
				KV Inspector is currently unavailable.
				<br />
				See console/logs for more details.
			</Info>
		);
	}

	const filteredEntries = data?.entries.filter?.((entry) => {
		if (!search) return true;
		
		try {
			// Decode base64 key to search in it
			const decodedKey = atob(entry.key);
			return decodedKey.toLowerCase().includes(search.toLowerCase());
		} catch {
			// If decode fails, search in the base64 string itself
			return entry.key.toLowerCase().includes(search.toLowerCase());
		}
	});

	if (filteredEntries?.length === 0) {
		return <Info>No KV entries found.</Info>;
	}

	return filteredEntries?.map((entry, index) => {
		return <KvEntry {...entry} key={`${entry.key}-${index}`} />;
	});
}

interface KvEntryProps {
	key: string;
	value: string;
	updateTs: number;
}

function KvEntry(props: KvEntryProps) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		ref.current?.scrollIntoView({
			behavior: "smooth",
			block: "nearest",
		});
	}, [props]);

	let decodedKey = props.key;
	let decodedValue = props.value;
	let parsedValue: unknown = null;
	let valueSize = "0 B";

	try {
		decodedKey = atob(props.key);
	} catch {
		// Keep original if decode fails
	}

	try {
		const valueBytes = atob(props.value);
		decodedValue = valueBytes;
		valueSize = formatBytes(valueBytes.length);
		
		// Try to parse as JSON for better display
		try {
			parsedValue = JSON.parse(valueBytes);
		} catch {
			// Not JSON, keep as string
			parsedValue = valueBytes;
		}
	} catch {
		// Keep original if decode fails
	}

	return (
		<div
			ref={ref}
			className="grid grid-cols-subgrid col-span-full text-xs px-4 pr-4 py-2 border-b hover:bg-muted/50 transition-colors"
		>
			<div className="[[data-show-timestamps]_&]:block hidden text-muted-foreground">
				{format(new Date(props.updateTs), "HH:mm:ss.SSS")}
			</div>
			<div className="font-mono text-xs break-all">
				{decodedKey}
			</div>
			<div className="font-mono text-xs">
				<ActorObjectInspector data={parsedValue} />
			</div>
			<div className="text-muted-foreground">
				{valueSize}
			</div>
		</div>
	);
}

function Info({ children }: PropsWithChildren) {
	return (
		<div className="col-span-full flex items-center justify-center p-8 text-sm text-muted-foreground">
			{children}
		</div>
	);
}

function formatBytes(bytes: number): string {
	if (bytes === 0) return "0 B";
	const k = 1024;
	const sizes = ["B", "KB", "MB", "GB"];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return `${Number.parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
}
