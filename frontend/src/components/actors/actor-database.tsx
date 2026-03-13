import {
	Badge,
	Button,
	Flex,
	Input,
	ScrollArea,
	Textarea,
	WithTooltip,
} from "@/components";
import {
	faChevronLeft,
	faChevronRight,
	faCode,
	faPlay,
	faRefresh,
	faTable,
	faTableCells,
	Icon,
} from "@rivet-gg/icons";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ShimmerLine } from "../shimmer-line";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import {
	type DatabaseColumn,
	type DatabaseExecuteRequest,
	type DatabaseExecuteResult,
	actorInspectorQueriesKeys,
	useActorInspector,
} from "./actor-inspector-context";
import { DatabaseTable } from "./database/database-table";
import type { ActorId } from "./queries";

const PAGE_SIZE = 100;
const DEFAULT_SQL = [
	"SELECT id, value, created_at",
	"FROM test_data",
	"ORDER BY id DESC",
	"LIMIT 25;",
].join("\n");

interface ActorDatabaseProps {
	actorId: ActorId;
}

export function ActorDatabase({ actorId }: ActorDatabaseProps) {
	const [view, setView] = useState<"tables" | "sql">("tables");

	return (
		<div className="flex flex-1 min-h-0 flex-col">
			<div className="flex items-center gap-2 border-b px-2 py-2">
				<Button
					variant={view === "tables" ? "secondary" : "ghost"}
					size="sm"
					onClick={() => setView("tables")}
				>
					<Icon icon={faTable} />
					Tables
				</Button>
				<Button
					variant={view === "sql" ? "secondary" : "ghost"}
					size="sm"
					onClick={() => setView("sql")}
				>
					<Icon icon={faCode} />
					Query
				</Button>
			</div>
			{view === "tables" ? (
				<ActorDatabaseBrowser actorId={actorId} />
			) : (
				<ActorDatabaseSqlShell actorId={actorId} />
			)}
		</div>
	);
}

function ActorDatabaseBrowser({ actorId }: ActorDatabaseProps) {
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
		(current) => current.table.name === selectedTable,
	);

	const totalRows = currentTable?.records ?? 0;
	const totalPages = Math.max(1, Math.ceil(totalRows / PAGE_SIZE));
	const hasNextPage = page < totalPages - 1;
	const hasPrevPage = page > 0;

	return (
		<>
			<div className="flex justify-between items-center border-b gap-1 h-[45px]">
				<div className="border-r h-full">
					<TableSelect
						actorId={actorId}
						onSelect={(nextTable) => {
							setTable(nextTable);
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
									onClick={() => setPage((current) => current - 1)}
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
									onClick={() => setPage((current) => current + 1)}
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

function ActorDatabaseSqlShell({ actorId }: ActorDatabaseProps) {
	const actorInspector = useActorInspector();
	const queryClient = useQueryClient();
	const [sql, setSql] = useState(DEFAULT_SQL);
	const [propertyDrafts, setPropertyDrafts] = useState<
		Record<string, string>
	>({});
	const [bindingChangeToken, setBindingChangeToken] = useState(0);
	const [result, setResult] = useState<DatabaseExecuteResult | null>(null);
	const namedBindings = useMemo(() => {
		return extractNamedBindings(sql);
	}, [sql]);
	const hasNamedBindings = namedBindings.length > 0;
	const hasPositionalBindings = useMemo(() => {
		return sql.includes("?");
	}, [sql]);
	const hasMixedBindings = hasNamedBindings && hasPositionalBindings;

	useEffect(() => {
		setPropertyDrafts((current) => {
			const next: Record<string, string> = {};
			for (const name of namedBindings) {
				next[name] = current[name] ?? "";
			}
			return next;
		});
	}, [namedBindings]);

	const parsedProperties = useMemo(() => {
		const next: Record<string, unknown> = {};
		for (const name of namedBindings) {
			const parsed = parseBindingDraft(propertyDrafts[name] ?? "");
			if (parsed.error) {
				return {
					value: {} as Record<string, unknown>,
					error: `${name}: ${parsed.error}`,
				};
			}
			next[name] = parsed.value;
		}
		return {
			value: next,
			error: null,
		};
	}, [namedBindings, propertyDrafts]);

	const resultColumns = useMemo(() => {
		return createResultColumns(result?.rows ?? []);
	}, [result]);
	const resultRowsAreTable = useMemo(() => {
		return areObjectRows(result?.rows ?? []);
	}, [result]);

	const { mutateAsync, isPending, error } = useMutation(
		actorInspector.actorDatabaseExecuteMutation(actorId),
	);
	const bindingError = hasMixedBindings
		? "Mixing positional `?` bindings and named properties is not supported in Inspector. Use one binding style."
		: hasPositionalBindings
			? "Positional `?` bindings are only supported in the Inspector HTTP API. Use named properties in the UI."
			: null;
	const propertiesError = hasNamedBindings ? parsedProperties.error : null;
	const runSql = useCallback(async () => {
		const request: DatabaseExecuteRequest = {
			sql,
		};

		if (hasNamedBindings) {
			request.properties = parsedProperties.value;
		}

		const nextResult = await mutateAsync(request);
		setResult(nextResult);
		await queryClient.invalidateQueries({
			queryKey: actorInspectorQueriesKeys.actorDatabase(actorId),
		});
	}, [
		actorId,
		hasNamedBindings,
		mutateAsync,
		parsedProperties.value,
		queryClient,
		sql,
	]);
	const canRun =
		sql.trim() !== "" &&
		bindingError === null &&
		propertiesError === null;

	useEffect(() => {
		if (
			bindingChangeToken === 0 ||
			!hasNamedBindings ||
			result === null ||
			propertiesError !== null ||
			isPending
		) {
			return;
		}

		const timer = window.setTimeout(() => {
			setBindingChangeToken(0);
			void runSql();
		}, 250);
		return () => window.clearTimeout(timer);
	}, [
		bindingChangeToken,
		hasNamedBindings,
		isPending,
		propertiesError,
		result,
		runSql,
	]);

	return (
		<div className="flex flex-1 min-h-0 flex-col">
			<div className="border-b p-3">
				<div className="flex items-center justify-between gap-3">
					<div>
						<div className="text-sm font-medium">Manual SQL</div>
						<div className="text-xs text-muted-foreground">
							Run statements directly against this actor&apos;s
							SQLite database. Use `RETURNING` when you want
							mutation output.
						</div>
					</div>
					<Button
						size="sm"
						isLoading={isPending}
						disabled={!canRun}
						onClick={() => {
							void runSql();
						}}
					>
						<Icon icon={faPlay} />
						Run
					</Button>
				</div>
				<div className="mt-3">
					<Textarea
						value={sql}
						onChange={(event) => setSql(event.target.value)}
						onKeyDown={async (event) => {
							if (
								(event.metaKey || event.ctrlKey) &&
								event.key === "Enter" &&
								canRun &&
								!isPending
							) {
								event.preventDefault();
								await runSql();
							}
						}}
						className="min-h-36 font-mono-console text-xs"
					/>
				</div>
				{hasNamedBindings ? (
					<div className="mt-3 space-y-2">
						<div className="flex items-center gap-2 text-xs text-muted-foreground">
							<Badge variant="outline">Named properties</Badge>
							<span>
								Edit a property to re-run the query preview.
							</span>
						</div>
						<div className="grid gap-2 md:grid-cols-2">
							{namedBindings.map((name) => (
								<label
									key={name}
									className="flex flex-col gap-1 text-xs"
								>
									<span className="font-mono-console text-muted-foreground">
										{name}
									</span>
									<Input
										value={propertyDrafts[name] ?? ""}
										onChange={(event) => {
											const nextValue =
												event.target.value;
											setPropertyDrafts((current) => ({
												...current,
												[name]: nextValue,
											}));
											if (result !== null) {
												setBindingChangeToken(
													(current) => current + 1,
												);
											}
										}}
										className="font-mono-console text-xs"
										placeholder="value"
									/>
								</label>
							))}
						</div>
					</div>
				) : null}
				{bindingError ? (
					<div className="mt-2 text-xs text-destructive">
						{bindingError}
					</div>
				) : null}
				{propertiesError ? (
					<div className="mt-2 text-xs text-destructive">
						{propertiesError}
					</div>
				) : null}
				{error ? (
					<div className="mt-2 text-xs text-destructive">
						{error instanceof Error
							? error.message
							: "Failed to execute SQL."}
					</div>
				) : null}
			</div>
			<div className="flex-1 min-h-0 overflow-hidden relative">
				{isPending ? <ShimmerLine /> : null}
				<div className="flex items-center justify-between gap-2 border-b px-3 py-2">
					<div className="text-xs text-muted-foreground">
						{result ? (
							<>
								Returned{" "}
								<span className="tabular-nums text-foreground">
									{result.rows.length}
								</span>{" "}
								row{result.rows.length === 1 ? "" : "s"}
							</>
						) : null}
					</div>
					{result ? (
						<Button
							variant="ghost"
							size="sm"
							onClick={() => setResult(null)}
						>
							Clear output
						</Button>
					) : null}
				</div>
				<ScrollArea className="h-full w-full">
					{result === null ? null : result.rows.length === 0 ? (
						<div className="px-3 py-4 text-sm text-muted-foreground">
							Statement executed successfully. No rows were
							returned.
						</div>
					) : resultRowsAreTable && resultColumns.length > 0 ? (
						<DatabaseTable
							className="overflow-hidden"
							columns={resultColumns}
							enableColumnResizing={false}
							enableRowSelection={false}
							enableSorting={false}
							data={result.rows}
						/>
					) : (
						<pre className="overflow-auto px-3 py-4 text-xs">
							{JSON.stringify(result.rows, null, 2)}
						</pre>
					)}
				</ScrollArea>
			</div>
		</div>
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
					<SelectItem disabled value="empty">
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

function areObjectRows(rows: unknown[]): rows is Record<string, unknown>[] {
	return rows.every(
		(row) =>
			row !== null &&
			typeof row === "object" &&
			!Array.isArray(row),
	);
}

function createResultColumns(rows: unknown[]): DatabaseColumn[] {
	if (!areObjectRows(rows)) {
		return [];
	}

	const names = Array.from(
		new Set(rows.flatMap((row) => Object.keys(row))),
	);
	return names.map((name, cid) => ({
		cid,
		name,
		type: "",
		notnull: false,
		dflt_value: null,
		pk: false,
	}));
}

function extractNamedBindings(sql: string): string[] {
	const matches = sql.matchAll(/([:@$])([A-Za-z_][A-Za-z0-9_]*)/g);
	return Array.from(new Set(Array.from(matches, (match) => match[2])));
}

function parseBindingDraft(value: string): {
	value: unknown;
	error: string | null;
} {
	if (value.trim() === "") {
		return { value: "", error: null };
	}

	try {
		const parsed = JSON.parse(value);
		if (!isSupportedBindingValue(parsed)) {
			return {
				value: "",
				error: "SQLite bindings must be null, number, string, or arrays of numbers.",
			};
		}
		return { value: parsed, error: null };
	} catch {
		return { value, error: null };
	}
}

function isSupportedBindingValue(value: unknown): boolean {
	if (
		value === null ||
		typeof value === "number" ||
		typeof value === "string"
	) {
		return true;
	}

	if (Array.isArray(value)) {
		return value.every((item) => typeof item === "number");
	}

	return false;
}
