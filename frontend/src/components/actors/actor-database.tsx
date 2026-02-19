import { Button, Flex, ScrollArea, WithTooltip } from "@/components";
import {
	faChevronLeft,
	faChevronRight,
	faRefresh,
	faTable,
	faTableCells,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { ShimmerLine } from "../shimmer-line";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import { useActorInspector } from "./actor-inspector-context";
import { DatabaseTable } from "./database/database-table";
import type { ActorId } from "./queries";

const PAGE_SIZE = 100;

interface ActorDatabaseProps {
	actorId: ActorId;
}

export function ActorDatabase({ actorId }: ActorDatabaseProps) {
	const actorInspector = useActorInspector();
	const { data, refetch } = useQuery(
		actorInspector.actorDatabaseQueryOptions(actorId),
	);
	const [table, setTable] = useState<string | undefined>(
		() => data?.tables?.[0]?.table.name,
	);
	const [page, setPage] = useState(0);

	const selectedTable = table || data?.tables?.[0]?.table.name;

	const {
		data: rows,
		refetch: refetchData,
		isLoading,
	} = useQuery({
		...actorInspector.actorDatabaseRowsQueryOptions(
			actorId,
			selectedTable ?? "",
			page,
			PAGE_SIZE,
		),
		enabled: !!selectedTable,
	});

	const currentTable = data?.tables?.find(
		(t) => t.table.name === selectedTable,
	);

	const totalRows = currentTable?.records ?? 0;
	const totalPages = Math.max(1, Math.ceil(totalRows / PAGE_SIZE));
	const hasNextPage = page < totalPages - 1;
	const hasPrevPage = page > 0;

	return (
		<>
			<div className="flex justify-between items-center border-b gap-1 h-[45px]">
				<div className="border-r h-full ">
					<TableSelect
						actorId={actorId}
						onSelect={(t) => {
							setTable(t);
							setPage(0);
						}}
						value={selectedTable}
					/>
				</div>
				<div className="flex-1 text-xs">
					<Flex className="items-center gap-2 h-full px-2">
						<Icon icon={faTableCells} />
						{currentTable ? (
							<>
								{currentTable.table.schema}.
								{currentTable.table.name}
								<span className="text-muted-foreground">
									({currentTable.columns.length} columns,{" "}
									{currentTable.records} rows)
								</span>
							</>
						) : (
							<span className="text-muted-foreground">
								No table selected
							</span>
						)}
					</Flex>
				</div>
				<div className="border-l h-full flex items-center gap-2 px-2">
					<div className="flex items-center gap-1">
						<WithTooltip
							content="Previous page"
							trigger={
								<Button
									variant="ghost"
									size="icon-sm"
									disabled={!hasPrevPage}
									onClick={() => setPage((p) => p - 1)}
								>
									<Icon icon={faChevronLeft} />
								</Button>
							}
						/>
						<span className="text-xs text-muted-foreground tabular-nums">
							{page + 1} / {totalPages}
						</span>
						<WithTooltip
							content="Next page"
							trigger={
								<Button
									variant="ghost"
									size="icon-sm"
									disabled={!hasNextPage}
									onClick={() => setPage((p) => p + 1)}
								>
									<Icon icon={faChevronRight} />
								</Button>
							}
						/>
					</div>
					<WithTooltip
						content="Refresh"
						trigger={
							<Button
								variant="ghost"
								size="icon-sm"
								isLoading={isLoading}
								onClick={() => {
									refetch();
									refetchData();
								}}
							>
								<Icon icon={faRefresh} />
							</Button>
						}
					/>
				</div>
			</div>
			<div className="flex-1 min-h-0 overflow-hidden flex relative">
				{isLoading ? <ShimmerLine /> : null}
				<ScrollArea className="w-full h-full min-h-0">
					{currentTable ? (
						<DatabaseTable
							className="overflow-hidden"
							columns={currentTable.columns}
							enableColumnResizing={false}
							enableRowSelection={false}
							data={rows ?? []}
							references={currentTable.foreignKeys}
						/>
					) : null}
				</ScrollArea>
			</div>
		</>
	);
}

function TableSelect({
	actorId,
	value,
	onSelect,
}: {
	actorId: ActorId;
	onSelect: (table: string) => void;
	value: string | undefined;
}) {
	const actorInspector = useActorInspector();
	const { data: tables } = useQuery(
		actorInspector.actorDatabaseTablesQueryOptions(actorId),
	);

	return (
		<Select onValueChange={onSelect} value={value}>
			<SelectTrigger variant="ghost" className="h-full pr-2 rounded-none">
				<SelectValue placeholder="Select table or view..." />
			</SelectTrigger>
			<SelectContent>
				{tables?.length === 0 ? (
					<SelectItem disabled value={"empty"}>
						<Flex className="items-center gap-2">
							<Icon icon={faTable} className="text-foreground" />
							No tables found
						</Flex>
					</SelectItem>
				) : null}
				{tables?.map((table) => (
					<SelectItem key={table.name} value={table.name}>
						<div className="flex items-center gap-2">
							<Icon icon={faTable} className="text-foreground" />
							{table.name}
						</div>
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
