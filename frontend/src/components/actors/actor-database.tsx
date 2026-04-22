import {
	faChevronLeft,
	faChevronRight,
	faPencil,
	faPlay,
	faRefresh,
	faTable,
	faTableCells,
	Icon,
} from "@rivet-gg/icons";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import equal from "fast-deep-equal";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
	Badge,
	Button,
	Flex,
	Input,
	ScrollArea,
	WithTooltip,
} from "@/components";
import {
	CodeMirror,
	keymap,
	Prec,
	type SQLConfig,
	sql,
} from "@/components/code-mirror";
import { ShimmerLine } from "../shimmer-line";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";
import {
	actorInspectorQueriesKeys,
	type DatabaseColumn,
	type DatabaseExecuteRequest,
	type DatabaseExecuteResult,
	useActorInspector,
} from "./actor-inspector-context";
import {
	DatabaseTable,
	type DatabaseTableCellContext,
	isBlobColumn,
	renderDatabaseCellValue,
} from "./database/database-table";
import type { ActorId } from "./queries";

const PAGE_SIZE = 100;

interface ActorDatabaseProps {
	actorId: ActorId;
}

type DatabaseBrowserRow = Record<string, unknown>;

type EditingCell = {
	rowKey: string;
	columnName: string;
};

type StagedCellEdit = {
	id: string;
	rowKey: string;
	columnName: string;
	primaryKeys: Array<{ name: string; value: unknown }>;
	originalValue: unknown;
	nextValue: unknown;
	draft: string;
};

function buildSelectSql(tableName: string, offset = 0): string {
	return `SELECT *\nFROM "${quoteSqlIdentifier(tableName)}"\nLIMIT ${PAGE_SIZE} OFFSET ${offset};`;
}

export function ActorDatabase({ actorId }: ActorDatabaseProps) {
	const actorInspector = useActorInspector();
	const queryClient = useQueryClient();

	const { data: schemaData, refetch: refetchSchema } = useQuery(
		actorInspector.actorDatabaseQueryOptions(actorId),
	);

	const [sql_text, setSqlText] = useState(() => "");
	const [editableTable, setEditableTable] = useState<string | null>(null);
	const [page, setPage] = useState(0);
	const [isAutoMode, setIsAutoMode] = useState(false);
	const [propertyDrafts, setPropertyDrafts] = useState<
		Record<string, string>
	>({});
	const [bindingChangeToken, setBindingChangeToken] = useState(0);
	const [result, setResult] = useState<DatabaseExecuteResult | null>(null);
	const [editingCell, setEditingCell] = useState<EditingCell | null>(null);
	const [editingValue, setEditingValue] = useState("");
	const [stagedEdits, setStagedEdits] = useState<
		Record<string, StagedCellEdit>
	>({});
	const [isApplyingEdits, setIsApplyingEdits] = useState(false);
	const [tableEditError, setTableEditError] = useState<string | null>(null);
	const [pendingRunConfirm, setPendingRunConfirm] = useState(false);

	const tables = schemaData?.tables ?? [];

	const sqlSchema = useMemo<SQLConfig["schema"]>(() => {
		const schema: Record<string, string[]> = {};
		for (const t of tables) {
			schema[t.table.name] = t.columns.map((c) => c.name);
		}
		return schema;
	}, [tables]);

	const handleRunRef = useRef<() => Promise<void> | void>(async () => {});

	const sqlExtensions = useMemo(
		() => [
			sql({ schema: sqlSchema, upperCaseKeywords: false }),
			Prec.highest(
				keymap.of([
					{
						key: "Mod-Enter",
						run: () => {
							void handleRunRef.current();
							return true;
						},
					},
				]),
			),
		],
		[sqlSchema],
	);

	const currentTableInfo = useMemo(() => {
		if (!editableTable) return null;
		return tables.find((t) => t.table.name === editableTable) ?? null;
	}, [editableTable, tables]);

	const primaryKeyColumns = useMemo(() => {
		return [...(currentTableInfo?.columns ?? [])]
			.filter((column) => Boolean(column.pk))
			.sort(
				(a, b) =>
					Number(a.pk ?? Number.MAX_SAFE_INTEGER) -
					Number(b.pk ?? Number.MAX_SAFE_INTEGER),
			);
	}, [currentTableInfo]);

	const canEditRows =
		currentTableInfo?.table.type === "table" &&
		primaryKeyColumns.length > 0;

	const resultColumns = useMemo(
		() => createResultColumns(result?.rows ?? []),
		[result],
	);
	const resultRowsAreTable = useMemo(
		() => areObjectRows(result?.rows ?? []),
		[result],
	);
	const visibleRows = useMemo(() => {
		if (!result) return [];
		return (result.rows ?? []).filter(isDatabaseBrowserRow);
	}, [result]);

	const rowLookup = useMemo(() => {
		const next = new Map<string, DatabaseBrowserRow>();
		for (const row of visibleRows) {
			const rowKey = createRowKey(row, primaryKeyColumns);
			if (rowKey) next.set(rowKey, row);
		}
		return next;
	}, [primaryKeyColumns, visibleRows]);

	const columnLookup = useMemo(() => {
		return new Map(
			(currentTableInfo?.columns ?? []).map(
				(column) => [column.name, column] as const,
			),
		);
	}, [currentTableInfo?.columns]);

	const namedBindings = useMemo(
		() => extractNamedBindings(sql_text),
		[sql_text],
	);
	const hasNamedBindings = namedBindings.length > 0;
	const hasPositionalBindings = useMemo(
		() => sql_text.includes("?"),
		[sql_text],
	);
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
		return { value: next, error: null };
	}, [namedBindings, propertyDrafts]);

	const bindingError = hasMixedBindings
		? "Mixing positional `?` bindings and named properties is not supported in Inspector. Use one binding style."
		: hasPositionalBindings
			? "Positional `?` bindings are only supported in the Inspector HTTP API. Use named properties in the UI."
			: null;
	const propertiesError = hasNamedBindings ? parsedProperties.error : null;

	const {
		mutateAsync,
		isPending,
		error: sqlError,
	} = useMutation(actorInspector.actorDatabaseExecuteMutation(actorId));

	const canRun =
		sql_text.trim() !== "" &&
		bindingError === null &&
		propertiesError === null;

	const stagedEditList = useMemo(
		() => Object.values(stagedEdits),
		[stagedEdits],
	);
	const stagedEditCount = stagedEditList.length;

	const executeRun = useCallback(async () => {
		if (!canRun || isPending) return;
		const request: DatabaseExecuteRequest = { sql: sql_text };
		if (hasNamedBindings) request.properties = parsedProperties.value;
		const nextResult = await mutateAsync(request);
		setResult(nextResult);
		const detectedTable = detectEditableTable(
			sql_text,
			tables,
			nextResult.rows,
		);
		setEditableTable(detectedTable);
		if (!detectedTable) {
			setStagedEdits({});
			setEditingCell(null);
			setEditingValue("");
		}
		await queryClient.invalidateQueries({
			queryKey: actorInspectorQueriesKeys.actorDatabase(actorId),
		});
	}, [
		actorId,
		canRun,
		hasNamedBindings,
		isPending,
		mutateAsync,
		parsedProperties.value,
		queryClient,
		sql_text,
		tables,
	]);

	const handleRun = useCallback(() => {
		if (stagedEditCount > 0) {
			setPendingRunConfirm(true);
			return;
		}
		void executeRun();
	}, [executeRun, stagedEditCount]);

	handleRunRef.current = handleRun;

	const totalRows = currentTableInfo?.records ?? 0;
	const totalPages = Math.max(1, Math.ceil(totalRows / PAGE_SIZE));

	const handlePageChange = useCallback(
		(nextPage: number) => {
			if (!editableTable) return;
			setPage(nextPage);
			const nextSql = buildSelectSql(editableTable, nextPage * PAGE_SIZE);
			setSqlText(nextSql);
			mutateAsync({ sql: nextSql })
				.then((r) => setResult(r))
				.catch(() => {});
		},
		[editableTable, mutateAsync],
	);

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
			void executeRun();
		}, 250);
		return () => window.clearTimeout(timer);
	}, [
		bindingChangeToken,
		hasNamedBindings,
		isPending,
		propertiesError,
		result,
		executeRun,
	]);

	const selectTable = useCallback(
		(tableName: string) => {
			setEditableTable(tableName);
			const initialSql = buildSelectSql(tableName, 0);
			setSqlText(initialSql);
			setResult(null);
			setEditingCell(null);
			setEditingValue("");
			setStagedEdits({});
			setTableEditError(null);
			setPage(0);
			setIsAutoMode(true);
			const request: DatabaseExecuteRequest = { sql: initialSql };
			mutateAsync(request)
				.then((r) => setResult(r))
				.catch(() => {});
		},
		[mutateAsync],
	);

	const hasAutoSelected = useRef(false);
	useEffect(() => {
		const firstName = tables[0]?.table.name;
		if (!firstName || hasAutoSelected.current) return;
		hasAutoSelected.current = true;
		selectTable(firstName);
	}, [tables, selectTable]);

	const handleSqlChange = useCallback(
		(value: string) => {
			setSqlText(value);
			setTableEditError(null);
			setIsAutoMode(false);
			if (stagedEditCount > 0) {
				setPendingRunConfirm(true);
			} else {
				setStagedEdits({});
			}
		},
		[stagedEditCount],
	);

	const beginCellEdit = useCallback(
		({ column, row, value }: DatabaseTableCellContext) => {
			if (!canEditRows || isBlobColumn(column, value)) return;
			const rowKey = createRowKey(row, primaryKeyColumns);
			if (!rowKey) return;
			const editId = createStagedEditId(rowKey, column.name);
			setEditingCell({ rowKey, columnName: column.name });
			setEditingValue(
				stagedEdits[editId]?.draft ?? formatCellDraft(value),
			);
			setTableEditError(null);
		},
		[canEditRows, primaryKeyColumns, stagedEdits],
	);

	const commitCellEdit = useCallback(() => {
		if (!editingCell) return;
		const row = rowLookup.get(editingCell.rowKey);
		const column = columnLookup.get(editingCell.columnName);
		if (!row || !column) {
			setEditingCell(null);
			setEditingValue("");
			return;
		}
		const nextValue = parseEditedCellValue(
			editingValue,
			row[column.name],
			column,
		);
		const editId = createStagedEditId(editingCell.rowKey, column.name);
		const primaryKeys = extractPrimaryKeyValues(row, primaryKeyColumns);
		if (primaryKeys.length !== primaryKeyColumns.length) {
			setEditingCell(null);
			setEditingValue("");
			return;
		}
		setStagedEdits((current) => {
			if (areDatabaseValuesEqual(nextValue, row[column.name])) {
				if (!(editId in current)) return current;
				const next = { ...current };
				delete next[editId];
				return next;
			}
			return {
				...current,
				[editId]: {
					id: editId,
					rowKey: editingCell.rowKey,
					columnName: column.name,
					primaryKeys,
					originalValue: row[column.name],
					nextValue,
					draft: editingValue,
				},
			};
		});
		setEditingCell(null);
		setEditingValue("");
	}, [columnLookup, editingCell, editingValue, primaryKeyColumns, rowLookup]);

	const discardEdits = useCallback(() => {
		setEditingCell(null);
		setEditingValue("");
		setStagedEdits({});
		setTableEditError(null);
	}, []);

	const applyEdits = useCallback(async () => {
		if (!editableTable || stagedEditList.length === 0) return;
		setIsApplyingEdits(true);
		setTableEditError(null);
		try {
			for (const edit of stagedEditList) {
				const properties: Record<string, unknown> = {
					set_val: edit.nextValue,
				};
				const whereClauses = edit.primaryKeys.map((primaryKey, i) => {
					const columnName = quoteSqlIdentifier(primaryKey.name);
					if (primaryKey.value === null)
						return `"${columnName}" IS NULL`;
					const propKey = `pk_${i}`;
					properties[propKey] = primaryKey.value;
					return `"${columnName}" = :${propKey}`;
				});
				await mutateAsync({
					sql: `UPDATE "${quoteSqlIdentifier(editableTable)}" SET "${quoteSqlIdentifier(edit.columnName)}" = :set_val WHERE ${whereClauses.join(" AND ")}`,
					properties,
				});
			}
			setEditingCell(null);
			setEditingValue("");
			setStagedEdits({});
			await Promise.all([
				refetchSchema(),
				queryClient.invalidateQueries({
					queryKey: actorInspectorQueriesKeys.actorDatabase(actorId),
				}),
			]);
			const refreshRequest: DatabaseExecuteRequest = { sql: sql_text };
			if (hasNamedBindings)
				refreshRequest.properties = parsedProperties.value;
			const refreshResult = await mutateAsync(refreshRequest);
			setResult(refreshResult);
			const detectedTable = detectEditableTable(
				sql_text,
				tables,
				refreshResult.rows,
			);
			setEditableTable(detectedTable);
			if (!detectedTable) {
				setStagedEdits({});
				setEditingCell(null);
				setEditingValue("");
			}
		} catch (error) {
			setTableEditError(
				error instanceof Error
					? error.message
					: "Failed to update edited cells.",
			);
		} finally {
			setIsApplyingEdits(false);
		}
	}, [
		actorId,
		editableTable,
		hasNamedBindings,
		mutateAsync,
		parsedProperties.value,
		queryClient,
		refetchSchema,
		sql_text,
		stagedEditList,
		tables,
	]);

	const renderBrowserCell = useCallback(
		(context: DatabaseTableCellContext) => {
			if (!canEditRows)
				return renderDatabaseCellValue(context.column, context.value);
			const rowKey = createRowKey(context.row, primaryKeyColumns);
			if (!rowKey)
				return renderDatabaseCellValue(context.column, context.value);

			const editId = createStagedEditId(rowKey, context.column.name);
			const stagedEdit = stagedEdits[editId];
			if (
				editingCell?.rowKey === rowKey &&
				editingCell.columnName === context.column.name
			) {
				return (
					<input
						autoFocus
						value={editingValue}
						onChange={(event) =>
							setEditingValue(event.target.value)
						}
						onBlur={commitCellEdit}
						onKeyDown={(event) => {
							if (event.key === "Enter") {
								event.preventDefault();
								commitCellEdit();
							} else if (event.key === "Escape") {
								event.preventDefault();
								setEditingCell(null);
								setEditingValue("");
							}
						}}
						className="h-8 font-mono-console text-sm bg-transparent border-0 outline-none"
					/>
				);
			}
			return renderDatabaseCellValue(
				context.column,
				stagedEdit ? stagedEdit.nextValue : context.value,
			);
		},
		[
			canEditRows,
			commitCellEdit,
			editingCell,
			editingValue,
			primaryKeyColumns,
			stagedEdits,
		],
	);

	const getBrowserCellClassName = useCallback(
		(context: DatabaseTableCellContext) => {
			if (!canEditRows) return undefined;
			const rowKey = createRowKey(context.row, primaryKeyColumns);
			if (!rowKey) return undefined;
			const editId = createStagedEditId(rowKey, context.column.name);
			if (stagedEdits[editId])
				return "bg-primary/10 ring-1 ring-inset ring-primary/35";
			if (
				editingCell?.rowKey === rowKey &&
				editingCell.columnName === context.column.name
			) {
				return "bg-primary/15 ring-1 ring-inset ring-primary/45";
			}
			if (!isBlobColumn(context.column, context.value))
				return "cursor-text";
			return undefined;
		},
		[canEditRows, editingCell, primaryKeyColumns, stagedEdits],
	);

	return (
		<div className="flex flex-1 min-h-0 flex-col">
			<div className="flex items-center border-b gap-1 h-[45px]">
				<TableSelect
					actorId={actorId}
					onSelect={selectTable}
					value={editableTable ?? undefined}
				/>
			</div>

			<div className="border-b px-3 py-3 space-y-3">
				<CodeMirror
					value={sql_text}
					onChange={handleSqlChange}
					extensions={sqlExtensions}
					className="min-h-20 text-xs rounded border border-border overflow-hidden"
					basicSetup={{
						lineNumbers: true,
						foldGutter: false,
						searchKeymap: false,
					}}
				/>
				{hasNamedBindings ? (
					<div className="space-y-2">
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
				{pendingRunConfirm && stagedEditCount > 0 ? (
					<div className="flex items-center gap-2 rounded border border-warning/40 bg-warning/10 px-3 py-2 text-xs text-warning">
						<span className="flex-1">
							You have {stagedEditCount} unsaved edit
							{stagedEditCount === 1 ? "" : "s"}. Discard and run?
						</span>
						<Button
							size="sm"
							variant="ghost"
							onClick={() => setPendingRunConfirm(false)}
						>
							Cancel
						</Button>
						<Button
							size="sm"
							onClick={() => {
								setPendingRunConfirm(false);
								discardEdits();
								void executeRun();
							}}
						>
							Discard & Run
						</Button>
					</div>
				) : null}
				<div className="flex items-center justify-between gap-2">
					<div className="flex items-center gap-2">
						<Button
							size="sm"
							isLoading={isPending}
							disabled={!canRun}
							onClick={() => void handleRun()}
						>
							<Icon icon={faPlay} />
							Run
						</Button>
						{result !== null ? (
							<Button
								variant="ghost"
								size="sm"
								onClick={() => setResult(null)}
							>
								Clear
							</Button>
						) : null}
						{stagedEditCount > 0 ? (
							<>
								<div className="h-7 w-px bg-border" />
								<Button
									variant="ghost"
									size="sm"
									disabled={isApplyingEdits}
									onClick={discardEdits}
								>
									Discard
								</Button>
								<Button
									size="sm"
									isLoading={isApplyingEdits}
									onClick={() => void applyEdits()}
								>
									Update {stagedEditCount} Cell
									{stagedEditCount === 1 ? "" : "s"}
								</Button>
							</>
						) : null}
					</div>
					<div className="flex items-center gap-2">
						<span className="text-xs text-muted-foreground">
							Cmd+Enter to run
						</span>
						<WithTooltip
							content="Refresh"
							trigger={
								<Button
									variant="ghost"
									size="icon-sm"
									isLoading={isPending}
									onClick={() => {
										refetchSchema();
										if (canRun) void handleRun();
									}}
								>
									<Icon icon={faRefresh} />
								</Button>
							}
						/>
					</div>
				</div>
				{bindingError ? (
					<div className="text-xs text-destructive">
						{bindingError}
					</div>
				) : null}
				{propertiesError ? (
					<div className="text-xs text-destructive">
						{propertiesError}
					</div>
				) : null}
				{sqlError ? (
					<div className="text-xs text-destructive">
						{sqlError instanceof Error
							? sqlError.message
							: "Failed to execute SQL."}
					</div>
				) : null}
			</div>

			<div className="flex-1 min-h-0 overflow-hidden flex flex-col relative">
				{isPending ? <ShimmerLine /> : null}
				{canEditRows ? (
					<div className="px-3 py-1.5 flex items-center justify-end">
						<WithTooltip
							content="Double-click any cell to edit inline. Only available when the query targets a single table with a primary key — modifying the SQL may disable it."
							trigger={
								<Badge
									variant="secondary"
									className="gap-1 text-xs cursor-default"
								>
									<Icon icon={faPencil} className="size-3" />
									Editable
								</Badge>
							}
						/>
					</div>
				) : null}
				{isAutoMode && totalPages > 1 ? (
					<div className="flex items-center justify-center gap-2 border-b px-3 py-1.5">
						<Button
							variant="ghost"
							size="icon-sm"
							disabled={page === 0 || isPending}
							onClick={() => handlePageChange(page - 1)}
						>
							<Icon icon={faChevronLeft} />
						</Button>
						<span className="text-xs text-muted-foreground">
							{page + 1} / {totalPages}
						</span>
						<Button
							variant="ghost"
							size="icon-sm"
							disabled={page >= totalPages - 1 || isPending}
							onClick={() => handlePageChange(page + 1)}
						>
							<Icon icon={faChevronRight} />
						</Button>
					</div>
				) : null}
				<ScrollArea className="w-full flex-1 min-h-0">
					{tableEditError ? (
						<div className="border-b px-3 py-2 text-xs text-destructive">
							{tableEditError}
						</div>
					) : null}
					{result === null ? (
						<div className="px-3 py-6 text-sm text-muted-foreground text-center">
							Select a table or run a query to see results.
						</div>
					) : result.rows.length === 0 ? (
						<div className="px-3 py-4 text-sm text-muted-foreground">
							Statement executed successfully. No rows were
							returned.
						</div>
					) : resultRowsAreTable && resultColumns.length > 0 ? (
						<DatabaseTable
							className="overflow-hidden"
							columns={
								canEditRows
									? (currentTableInfo?.columns ??
										resultColumns)
									: resultColumns
							}
							enableColumnResizing={false}
							enableRowSelection={false}
							enableSorting={false}
							data={visibleRows}
							references={
								canEditRows
									? (currentTableInfo?.foreignKeys ?? [])
									: []
							}
							renderCell={
								canEditRows ? renderBrowserCell : undefined
							}
							getCellClassName={
								canEditRows
									? getBrowserCellClassName
									: undefined
							}
							onCellDoubleClick={
								canEditRows ? beginCellEdit : undefined
							}
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

function detectEditableTable(
	sqlText: string,
	tables: Array<{
		table: { name: string; type: string };
		columns: DatabaseColumn[];
	}>,
	rows: unknown[],
): string | null {
	if (!areObjectRows(rows) || rows.length === 0) return null;

	// Bail out on JOINs — ambiguous which table to update.
	if (/\bJOIN\b/i.test(sqlText)) return null;

	const fromMatch = sqlText.match(/\bFROM\s+(?:"([^"]+)"|`([^`]+)`|(\w+))/i);
	if (!fromMatch) return null;
	const tableName = fromMatch[1] ?? fromMatch[2] ?? fromMatch[3];
	if (!tableName) return null;

	const tableInfo = tables.find(
		(t) => t.table.name === tableName && t.table.type === "table",
	);
	if (!tableInfo) return null;

	const pks = tableInfo.columns.filter((c) => Boolean(c.pk));
	if (pks.length === 0) return null;

	const resultKeys = new Set(Object.keys(rows[0]));
	if (!pks.every((pk) => resultKeys.has(pk.name))) return null;

	return tableName;
}

function areObjectRows(rows: unknown[]): rows is Record<string, unknown>[] {
	return rows.every(
		(row) => row !== null && typeof row === "object" && !Array.isArray(row),
	);
}

function createResultColumns(rows: unknown[]): DatabaseColumn[] {
	if (!areObjectRows(rows)) return [];
	const names = Array.from(new Set(rows.flatMap((row) => Object.keys(row))));
	return names.map((name, cid) => ({
		cid,
		name,
		type: "",
		notnull: false,
		dflt_value: null,
		pk: false,
	}));
}

function isDatabaseBrowserRow(value: unknown): value is DatabaseBrowserRow {
	return value !== null && typeof value === "object" && !Array.isArray(value);
}

function createRowKey(
	row: DatabaseBrowserRow,
	primaryKeyColumns: DatabaseColumn[],
): string | null {
	if (primaryKeyColumns.length === 0) return null;
	const values = primaryKeyColumns.map((column) => {
		if (!(column.name in row)) return undefined;
		return [column.name, row[column.name]];
	});
	if (values.some((value) => value === undefined)) return null;
	return JSON.stringify(values);
}

function createStagedEditId(rowKey: string, columnName: string): string {
	return `${rowKey}:${columnName}`;
}

function extractPrimaryKeyValues(
	row: DatabaseBrowserRow,
	primaryKeyColumns: DatabaseColumn[],
) {
	return primaryKeyColumns.flatMap((column) => {
		if (!(column.name in row)) return [];
		return [{ name: column.name, value: row[column.name] }];
	});
}

function formatCellDraft(value: unknown): string {
	if (value === null) return "";
	if (typeof value === "string") return value;
	return String(value);
}

function parseEditedCellValue(
	draft: string,
	originalValue: unknown,
	column: DatabaseColumn,
): unknown {
	const trimmed = draft.trim();
	if (trimmed.toLowerCase() === "null") return null;
	const type = column.type.toLowerCase();
	if (
		typeof originalValue === "number" ||
		/\b(int|real|floa|doub|dec|num|bool)\b/.test(type)
	) {
		const numeric = Number(trimmed);
		if (trimmed !== "" && !Number.isNaN(numeric)) return numeric;
	}
	return draft;
}

function areDatabaseValuesEqual(a: unknown, b: unknown): boolean {
	return equal(a, b);
}

function quoteSqlIdentifier(value: string): string {
	return value.replace(/"/g, '""');
}

function extractNamedBindings(sql: string): string[] {
	const matches = sql.matchAll(/([:@$])([A-Za-z_][A-Za-z0-9_]*)/g);
	return Array.from(new Set(Array.from(matches, (match) => match[2])));
}

function parseBindingDraft(value: string): {
	value: unknown;
	error: string | null;
} {
	if (value.trim() === "") return { value: "", error: null };
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
	)
		return true;
	if (Array.isArray(value))
		return value.every((item) => typeof item === "number");
	return false;
}
