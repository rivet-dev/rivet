import {
	faAnglesUpDown,
	faArrowDownWideShort,
	faArrowUpWideShort,
	faLink,
	Icon,
} from "@rivet-gg/icons";
import type { Column, Columns, ForeignKeys } from "@rivetkit/core/inspector";
import {
	createColumnHelper,
	// SortingState,
	type ExpandedState,
	flexRender,
	getCoreRowModel,
	getExpandedRowModel,
	getSortedRowModel,
	type RowSelectionState,
	type SortingState,
	useReactTable as useTable,
} from "@tanstack/react-table";
import { Fragment, useMemo, useState } from "react";
import {
	Badge,
	Button,
	Checkbox,
	cn,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components";

interface DatabaseTableProps {
	columns: any[];
	data: any[];
	references?: ForeignKeys;
	className?: string;

	enableRowSelection?: boolean;
	enableSorting?: boolean;
	enableCellExpanding?: boolean;
	enableColumnResizing?: boolean;
}

export function DatabaseTable({
	columns: dbCols,
	data,
	references,
	className,

	enableRowSelection = true,
	enableSorting = true,
	enableCellExpanding = true,
	enableColumnResizing = true,
}: DatabaseTableProps) {
	const columns = useMemo(() => {
		return createColumns(dbCols, references, { enableRowSelection });
	}, [dbCols, references, enableRowSelection]);

	const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
	const [sorting, setSorting] = useState<SortingState>([]);
	const [expanded, setExpanded] = useState<ExpandedState>({});

	const table = useTable({
		columns,
		data,
		enableRowSelection,
		enableSorting,
		enableCellExpanding,
		enableColumnResizing,
		getCoreRowModel: getCoreRowModel(),
		getExpandedRowModel: getExpandedRowModel(),
		getSortedRowModel: getSortedRowModel(),
		defaultColumn: {},
		columnResizeMode: "onChange",
		onSortingChange: setSorting,
		onRowSelectionChange: setRowSelection,
		paginateExpandedRows: false,
		state: {
			sorting,
			rowSelection,
			expanded,
		},
	});

	function calculateColumnSizes() {
		const headers = table.getFlatHeaders();
		const colSizes: { [key: string]: number } = {};
		for (let i = 0; i < headers.length; i++) {
			const header = headers[i]!;
			colSizes[`--header-${header.id}-size`] = header.getSize();
			colSizes[`--col-${header.column.id}-size`] =
				header.column.getSize();
		}
		return colSizes;
	}

	const columnSizeVars = useMemo(() => {
		return calculateColumnSizes();
	}, [table.getState().columnSizingInfo, table.getState().columnSizing]);

	return (
		<Table
			containerClassName="overflow-visible"
			className={cn("w-auto", className)}
			style={{
				...columnSizeVars,
				width: table.getTotalSize(),
			}}
		>
			<TableHeader>
				{table.getHeaderGroups().map((headerGroup) => (
					<TableRow key={headerGroup.id}>
						{headerGroup.headers.map((header) => {
							return (
								<TableHead
									key={header.id}
									colSpan={header.colSpan}
									className="text-left min-h-0 h-auto border-r p-0 m-0 relative text-foreground"
								>
									{header.isPlaceholder ? null : header.column.getCanSort() ? (
										<Button
											variant="ghost"
											className="text-foreground px-2 py-2 rounded-none h-full items-center min-h-0 w-full justify-start min-w-52"
											style={{
												width: `calc(var(--header-${header?.id}-size) * 1px)`,
											}}
											onClick={header.column.getToggleSortingHandler()}
										>
											<span className="flex-1 min-w-0 text-left">
												{flexRender(
													header.column.columnDef
														.header,
													header.getContext(),
												)}
											</span>

											{header.column.getCanSort() ? (
												header.column.getIsSorted() ===
												"asc" ? (
													<Icon
														icon={
															faArrowUpWideShort
														}
													/>
												) : header.column.getIsSorted() ===
													"desc" ? (
													<Icon
														icon={
															faArrowDownWideShort
														}
													/>
												) : (
													<Icon
														icon={faAnglesUpDown}
													/>
												)
											) : null}
										</Button>
									) : (
										<div className="px-2 py-2">
											{flexRender(
												header.column.columnDef.header,
												header.getContext(),
											)}
										</div>
									)}
									{header.column.getCanResize() ? (
										<div
											className="cursor-col-resize select-none w-3 -mr-1.5 flex items-center justify-center absolute right-0 inset-y-0 group"
											onMouseDown={header.getResizeHandler()}
											onTouchStart={header.getResizeHandler()}
										>
											<div
												className={cn(
													"w-px h-full bg-transparent transition-colors group-hover:bg-primary/30",
													header.column.getIsResizing() &&
														"bg-primary",
												)}
											/>
										</div>
									) : null}
								</TableHead>
							);
						})}
					</TableRow>
				))}
			</TableHeader>
			<TableBody className="[&_tr:last-child]:border-px">
				{table.getRowModel().rows.map((row) => (
					<Fragment key={row.id}>
						<TableRow>
							{row.getVisibleCells().map((cell) => (
								<TableCell
									key={cell.id}
									className={cn(
										"p-2 border-r font-mono-console",
									)}
									style={{
										width: `calc(var(--col-${cell.column.id}-size) * 1px)`,
									}}
								>
									<div className="flex items-center gap-2">
										<div className="flex-1">
											{flexRender(
												cell.column.columnDef.cell,
												cell.getContext(),
											)}
										</div>
									</div>
								</TableCell>
							))}
						</TableRow>
					</Fragment>
				))}
			</TableBody>
		</Table>
	);
}

const ch = createColumnHelper();

function createColumns(
	columns: Columns,
	references?: ForeignKeys,
	{ enableRowSelection }: { enableRowSelection?: boolean } = {},
) {
	return [
		...[
			enableRowSelection
				? ch.display({
						id: "select",
						enableResizing: false,
						header: ({ table }) => (
							<Checkbox
								className="border-border data-[state=checked]:bg-secondary data-[state=indeterminate]:bg-secondary data-[state=checked]:text-primary-foreground block size-5"
								checked={
									table.getIsAllRowsSelected()
										? true
										: table.getIsSomeRowsSelected()
											? "indeterminate"
											: false
								}
								onCheckedChange={(value) => {
									if (value === "indeterminate") {
										table.toggleAllRowsSelected(true);
										return;
									}
									table.toggleAllRowsSelected(value);
								}}
								aria-label="Select all"
							/>
						),
						cell: ({ row }) => (
							<Checkbox
								className="border-border data-[state=checked]:bg-secondary data-[state=checked]:text-primary-foreground block size-5"
								checked={row.getIsSelected()}
								disabled={!row.getCanSelect()}
								onCheckedChange={(value) => {
									if (value === "indeterminate") {
										row.toggleSelected(true);
										return;
									}
									row.toggleSelected();
								}}
							/>
						),
					})
				: null,
		].filter(Boolean),
		...columns.map((col) =>
			ch.accessor(col.name, {
				header: (info) => (
					<span className="flex items-center gap-1">
						{col.name}{" "}
						<span className="text-muted-foreground text-xs  font-mono-console">
							{col.type}
						</span>
						<ForeignKey references={references} column={col} />
					</span>
				),
				cell: (info) => {
					if (col.type === "blob") {
						return (
							<span className="text-xs text-muted-foreground font-mono-console">
								BINARY{" "}
							</span>
						);
					}
					const value = info.getValue();
					if (value === null) {
						return (
							<span className="text-xs text-muted-foreground font-mono-console">
								NULL
							</span>
						);
					}

					if (col) return <>{info.getValue()}</>;
				},
				meta: {
					type: col.type,
					notNull: col.notnull,
					default: col.dflt_value,
				},
			}),
		),
	];
}

function ForeignKey({
	references,
	column,
}: {
	references?: ForeignKeys;
	column: Column;
}) {
	const ref = references?.find((ref) => ref.from === column.name);
	if (!ref) return null;
	return (
		<Badge variant="outline" className="text-xs ml-2">
			<Icon icon={faLink} className="mr-1" />
			{ref.table}.{ref.to}
		</Badge>
	);
}
