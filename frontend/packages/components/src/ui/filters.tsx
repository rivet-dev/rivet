"use client";
import { Checkbox } from "./checkbox";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
} from "./command";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "./dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";
import { cn } from "../lib/utils";
import {
	type IconProp,
	faCheck,
	faFilterList,
	faTimes as faX,
	Icon,
} from "@rivet-gg/icons";
import {
	type Dispatch,
	type SetStateAction,
	useRef,
	useState,
	useEffect,
	type ReactNode,
	Suspense,
	type FunctionComponent,
} from "react";
import { Button } from "./button";
import { motion } from "framer-motion";
import { Badge } from "./badge";
import { Calendar } from "./calendar";
import type { DateRange } from "react-day-picker";
import { object, z } from "zod";
import { endOfDay, lightFormat, startOfDay, subMonths } from "date-fns";
import { Input } from "./input";
import { Label } from "./label";
import _ from "lodash";

interface AnimateChangeInHeightProps {
	children: React.ReactNode;
	className?: string;
}

export const AnimateChangeInHeight: React.FC<AnimateChangeInHeightProps> = ({
	children,
	className,
}) => {
	const containerRef = useRef<HTMLDivElement | null>(null);
	const [height, setHeight] = useState<number | "auto">("auto");

	useEffect(() => {
		if (containerRef.current) {
			const resizeObserver = new ResizeObserver((entries) => {
				// We only have one entry, so we can use entries[0].
				const observedHeight = entries[0].contentRect.height;
				setHeight(observedHeight);
			});

			resizeObserver.observe(containerRef.current);

			return () => {
				// Cleanup the observer when the component is unmounted
				resizeObserver.disconnect();
			};
		}
	}, []);

	return (
		<motion.div
			className={cn(className, "overflow-hidden")}
			style={{ height }}
			animate={{ height }}
			transition={{ duration: 0.1, dampping: 0.2, ease: "easeIn" }}
		>
			<div ref={containerRef}>{children}</div>
		</motion.div>
	);
};

export enum FilterOp {
	EQUAL = "equal",
	NOT_EQUAL = "not",
	AFTER = "after",
	BEFORE = "before",
	BETWEEN = "between",
}

export type Filter = {
	operator: FilterOp;
	value: string[];
};

function filterDefinitionToOptions(definition: FilterDefinition) {
	if (definition.type === "select" && Array.isArray(definition.options)) {
		return definition.options.map((option) => ({
			value: option.value,
			label: option.label,
		}));
	}

	return [];
}

function defaultFilterDefinitionOperator({
	definition,
	filterValues,
}: { definition: FilterDefinition; filterValues: string[] }) {
	if (definition.type === "date") {
		return FilterOp.AFTER;
	}
	return FilterOp.EQUAL;
}

const filterOperators = ({
	definition,
	filterValues,
}: {
	definition: FilterDefinition;
	filterValues: string[];
}) => {
	switch (definition.type) {
		case "select":
			return [FilterOp.EQUAL, FilterOp.NOT_EQUAL];
		case "date":
			return [FilterOp.BETWEEN, FilterOp.AFTER, FilterOp.BEFORE];
		default:
			return [];
	}
};

const FilterOperatorLabel = ({
	valuesCount,
	operator,
	definition,
}: {
	valuesCount: number;
	operator: FilterOp;
	definition: FilterDefinition;
}) => {
	const customOperator = definition.operators?.[operator];
	if (customOperator) {
		return customOperator;
	}

	switch (operator) {
		case FilterOp.EQUAL:
			return valuesCount > 1 ? "is any of" : "is";
		case FilterOp.NOT_EQUAL:
			return valuesCount > 1 ? "is not any of" : "is not";
		case FilterOp.AFTER:
			return "is after";
		case FilterOp.BEFORE:
			return "is before";
		case FilterOp.BETWEEN:
			return "is between";
		default:
			return "";
	}
};

const FilterOperatorDropdown = ({
	definition,
	operator,
	filterValues,
	setOperator,
}: {
	definition: FilterDefinition;
	operator: FilterOp;
	filterValues: string[];
	setOperator: (operator: FilterOp) => void;
}) => {
	const operators = filterOperators({ definition, filterValues });
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="outline"
					size="xs"
					className="shrink-0 rounded-none px-1 border-y-0 ml-1 mr-0 h-full"
				>
					<FilterOperatorLabel
						valuesCount={filterValues.length}
						operator={operator}
						definition={definition}
					/>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="start" className="w-fit min-w-fit">
				{operators.map((operator) => (
					<DropdownMenuItem
						key={operator}
						onClick={() => setOperator(operator)}
					>
						<FilterOperatorLabel
							valuesCount={filterValues.length}
							operator={operator}
							definition={definition}
						/>
					</DropdownMenuItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
};

const FilterValueCombobox = ({
	id,
	definition,
	value,
	onChange,
}: {
	id: string;
	definition: FilterSelectDefinition;
	operator: FilterOp;
	value: string[];
	onChange: OnFiltersChange;
}) => {
	const [open, setOpen] = useState(false);
	const [commandInput, setCommandInput] = useState("");
	const commandInputRef = useRef<HTMLInputElement>(null);

	const selectedOptionsCount = value.length;

	const Display = definition.display;
	const display = Display ? (
		<Display value={value} />
	) : (
		<>
			{selectedOptionsCount === 1 ? (
				<span>{value[0]}</span>
			) : (
				<span>{selectedOptionsCount} selected</span>
			)}
		</>
	);
	return (
		<Popover
			open={open}
			onOpenChange={(open) => {
				setOpen(open);
				if (!open) {
					setTimeout(() => {
						setCommandInput("");
					}, 200);
				}
			}}
		>
			<PopoverTrigger asChild>
				<Button
					variant="outline"
					size="xs"
					className="shrink-0 rounded-none px-1 border-y-0 mx-1 border-l-0 mx-0 text-xs h-full"
				>
					{display}
				</Button>
			</PopoverTrigger>
			<PopoverContent className="p-0">
				<AnimateChangeInHeight>
					<Command>
						<CommandInput
							placeholder={definition.label}
							className="h-9"
							value={commandInput}
							onInputCapture={(e) => {
								setCommandInput(e.currentTarget.value);
							}}
							ref={commandInputRef}
						/>
						<CommandList>
							<CommandEmpty>No results found.</CommandEmpty>
							<FilterOptions
								definition={definition}
								value={value}
								onOptionSelect={(values) => {
									onChange((prev) => ({
										...prev,
										[id]: {
											...prev[id],
											value: values,
										},
									}));
								}}
							/>
						</CommandList>
					</Command>
				</AnimateChangeInHeight>
			</PopoverContent>
		</Popover>
	);
};

function FilterDateValueCombobox({
	id,
	definition,
	operator,
	value,
	onChange,
}: {
	id: string;
	definition: FilterDefinition;
	operator: FilterOp;
	value: string[];
	onChange: OnFiltersChange;
}) {
	const [open, setOpen] = useState(false);

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="outline"
					size="xs"
					className="shrink-0 rounded-none px-1 border-y-0 mx-1 border-l-0 mx-0 h-full"
				>
					{operator === FilterOp.BETWEEN
						? `${new Date(+value[0]).toLocaleString()} - ${new Date(+value[1]).toLocaleString()}`
						: `${new Date(+value[0]).toLocaleString()}`}
				</Button>
			</PopoverTrigger>
			<PopoverContent className="p-0">
				<AnimateChangeInHeight>
					<Command>
						<CommandList>
							<FilterDateOption
								operator={operator}
								value={value}
								onOptionSelect={(values) => {
									onChange((prev) => ({
										...prev,
										[id]: {
											...prev[id],
											value: values,
										},
									}));
								}}
							/>
						</CommandList>
					</Command>
				</AnimateChangeInHeight>
			</PopoverContent>
		</Popover>
	);
}

function FilterBooleanValue() {
	return (
		<Button
			variant="outline"
			size="xs"
			className="shrink-0 rounded-none px-1 border-y-0 mx-1 border-l-0 mx-0 text-xs shrink-0 cursor-default pointer-events-none "
		>
			<Icon icon={faCheck} />
		</Button>
	);
}

function FilterValue({
	id,
	definition,
	value,
	operator,
	onChange,
}: {
	id: string;
	definition: FilterDefinition;
	operator: FilterOp;
	value: string[];
	onChange: OnFiltersChange;
}) {
	if (definition.type === "boolean") {
		return <FilterBooleanValue />;
	}
	if (definition.type === "select") {
		return (
			<FilterValueCombobox
				id={id}
				operator={operator}
				definition={definition}
				value={value}
				onChange={onChange}
			/>
		);
	}

	if (definition.type === "date") {
		return (
			<FilterDateValueCombobox
				id={id}
				operator={operator}
				definition={definition}
				value={value}
				onChange={onChange}
			/>
		);
	}
}

function FilterOperator({
	definition,
	filter,
	id,
	onFiltersChange,
}: {
	definition: FilterDefinition;
	filter: Filter;
	id: string;
	onFiltersChange: OnFiltersChange;
}) {
	if (definition.type === "boolean") {
		return null;
	}
	return (
		<FilterOperatorDropdown
			definition={definition}
			operator={filter.operator}
			filterValues={filter.value}
			setOperator={(operator) => {
				if (definition.type === "date") {
					if (operator === FilterOp.BETWEEN) {
						return onFiltersChange((prev) => ({
							...prev,
							[id]: {
								...prev[id],
								operator,
								value: [
									String(
										prev[id].value?.[1] ??
											subMonths(
												startOfDay(Date.now()),
												1,
											).getTime(),
									),
									String(
										prev[id].value?.[0] ??
											endOfDay(Date.now()).getTime(),
									),
								],
							},
						}));
					}
					return onFiltersChange((prev) => ({
						...prev,
						[id]: {
							...prev[id],
							operator,
							value: [
								prev[id].value?.[1] ??
									prev[id].value?.[0] ??
									String(Date.now()),
							],
						},
					}));
				}

				onFiltersChange((prev) => ({
					...prev,
					[id]: {
						...prev[id],
						operator,
					},
				}));
			}}
		/>
	);
}

export default function Filters({
	filters,
	onFiltersChange,
	definitions,
}: {
	filters: Partial<Filters>;
	onFiltersChange: OnFiltersChange;
	definitions: FilterDefinitions;
}) {
	return (
		<div className="flex gap-2 flex-wrap">
			{Object.entries(filters)
				.filter(
					(entry): entry is [string, Filter] =>
						entry[1] !== undefined && entry[1].value?.length > 0,
				)
				.map(([key, filter]) => {
					const definition = definitions[key];

					if (!definition) return null;
					return (
						<Badge
							variant="outline"
							key={key}
							className="p-0 h-7 rounded-md"
						>
							<Icon
								icon={definition.icon}
								className="mr-1 ml-1.5"
							/>
							<span className="mr-0.5">{definition.label}</span>
							<FilterOperator
								id={key}
								definition={definition}
								filter={filter}
								onFiltersChange={onFiltersChange}
							/>
							<FilterValue
								id={key}
								definition={definition}
								value={filter.value}
								operator={filter.operator}
								onChange={onFiltersChange}
							/>
							<Button
								className="pl-1 pr-1.5 rounded-none h-full"
								variant="ghost"
								size="xs"
								onClick={() => {
									onFiltersChange((prev) => {
										return Object.fromEntries(
											Object.entries(prev).filter(
												([id]) => id !== key,
											),
										);
									});
								}}
							>
								<Icon icon={faX} />
							</Button>
						</Badge>
					);
				})}
		</div>
	);
}

export type FilterDefinition =
	| {
			type: "date" | "string" | "number" | "boolean";
			label: string;
			icon: IconProp;
			operators?: Partial<Record<FilterOp, string>>;
			display?: FunctionComponent<PreviewProviderProps>;
	  }
	| FilterSelectDefinition;

type FilterSelectDefinition =
	| FilterSelectStaticDefinition
	| FilterSelectDynamicDefinition;

type FilterSelectStaticDefinition = {
	type: "select";
	label: string;
	icon: IconProp;
	operators?: Partial<Record<FilterOp, string>>;
	options: { value: string; label: ReactNode }[];
	display?: FunctionComponent<PreviewProviderProps>;
};
type FilterSelectDynamicDefinition = {
	type: "select";
	label: string;
	icon: IconProp;
	operators?: Partial<Record<FilterOp, string>>;
	options: FunctionComponent<OptionsProviderProps>;
	display?: FunctionComponent<PreviewProviderProps>;
};
type Filters = Record<string, Filter>;
export type OnFiltersChange = Dispatch<SetStateAction<Filters>>;
export type FilterDefinitions = Record<string, FilterDefinition>;
type OnOptionSelect = (
	values: string[],
	opts?: { defaultOperator?: FilterOp; closeAfter?: boolean; defId?: string },
) => void;

export const FilterCreator = ({
	definitions,
	value,
	onChange,
}: {
	definitions: FilterDefinitions;
	value: Partial<Filters>;
	onChange: OnFiltersChange;
}) => {
	const [open, setOpen] = useState(false);
	const [selectedDefId, setSelectedDefId] = useState<string | null>(null);
	const [commandInput, setCommandInput] = useState("");
	const commandInputRef = useRef<HTMLInputElement>(null);

	const selectedDefinition = definitions[selectedDefId ?? ""] ?? null;

	const onOptionSelect: OnOptionSelect = (values, opts) => {
		const defId = opts?.defId ?? selectedDefId;
		const def = definitions[defId ?? ""];
		if (!defId || !def) {
			return;
		}
		onChange((prev) => ({
			...prev,
			[defId]: {
				operator:
					opts?.defaultOperator ??
					defaultFilterDefinitionOperator({
						definition: selectedDefinition,
						filterValues: values,
					}),
				value: values,
			},
		}));

		if (opts?.closeAfter) {
			setTimeout(() => {
				setSelectedDefId(null);
				setCommandInput("");
			}, 200);
			setOpen(false);
		}
	};

	return (
		<div className="flex gap-2 flex-wrap items-center">
			<Filters
				filters={value}
				onFiltersChange={onChange}
				definitions={definitions}
			/>
			{Object.values(value).filter(
				(filter): filter is Filter =>
					filter !== undefined && filter.value?.length > 0,
			).length > 0 && (
				<Button
					variant="outline"
					size="sm"
					onClick={() => onChange({})}
				>
					Clear
				</Button>
			)}
			<Popover
				open={open}
				onOpenChange={(open) => {
					setOpen(open);
					if (!open) {
						setTimeout(() => {
							setSelectedDefId(null);
							setCommandInput("");
						}, 200);
					}
				}}
			>
				<PopoverTrigger asChild>
					<Button
						variant="outline"
						// biome-ignore lint/a11y/useSemanticElements: <explanation>
						role="combobox"
						aria-expanded={open}
						size="sm"
						startIcon={<Icon icon={faFilterList} />}
					>
						{!value.length && "Filter"}
					</Button>
				</PopoverTrigger>
				<PopoverContent className="p-0">
					<AnimateChangeInHeight>
						<Command>
							<CommandList>
								{selectedDefinition?.type === "select" ||
								!selectedDefinition ? (
									<CommandInput
										placeholder={
											selectedDefinition
												? selectedDefinition.label
												: "Filter..."
										}
										className="h-9"
										value={commandInput}
										onInputCapture={(e) => {
											setCommandInput(
												e.currentTarget.value,
											);
										}}
										ref={commandInputRef}
									/>
								) : null}
								<CommandEmpty>No results found.</CommandEmpty>
								{selectedDefinition && selectedDefId ? (
									<FilterOptions
										value={
											value[selectedDefId]?.value ?? []
										}
										definition={selectedDefinition}
										onOptionSelect={onOptionSelect}
									/>
								) : (
									<>
										<CommandGroup>
											{Object.entries(definitions)
												.filter(
													([id]) =>
														value[id] === undefined,
												)
												.map(([id, definition]) => (
													<CommandItem
														key={id}
														className="group text-muted-foreground flex gap-2 items-center"
														value={id}
														onSelect={() => {
															if (
																definition.type ===
																"boolean"
															) {
																onOptionSelect(
																	["true"],
																	{
																		defId: id,
																		defaultOperator:
																			FilterOp.EQUAL,
																		closeAfter: true,
																	},
																);
																setOpen(false);
																setCommandInput(
																	"",
																);
																return;
															}

															setSelectedDefId(
																id,
															);
															setCommandInput("");
															commandInputRef.current?.focus();
														}}
													>
														<Icon
															icon={
																definition.icon
															}
														/>
														<span className="text-accent-foreground">
															{definition.label}
														</span>
													</CommandItem>
												))}
										</CommandGroup>
									</>
								)}
							</CommandList>
						</Command>
					</AnimateChangeInHeight>
				</PopoverContent>
			</Popover>
		</div>
	);
};

function FilterOptions({
	value,
	definition,
	onOptionSelect,
}: {
	value: string[];
	definition: FilterDefinition;
	onOptionSelect: OnOptionSelect;
}) {
	if (definition.type === "select" && Array.isArray(definition.options)) {
		return (
			<CommandGroup>
				{filterDefinitionToOptions(definition).map((option) => {
					const isSelected = value.includes(option.value);
					return (
						<CommandItem
							className="group text-muted-foreground flex gap-2 items-center"
							key={option.value}
							value={option.value}
							onSelect={(currentValue) => {
								if (isSelected) {
									onOptionSelect(
										value.filter(
											(val) => val !== currentValue,
										),
										{
											closeAfter: true,
										},
									);
									return;
								}
								// If the option is not selected, add it to the value
								// and close the command

								onOptionSelect([...value, currentValue], {
									closeAfter: true,
								});
							}}
						>
							<Checkbox
								checked={isSelected}
								className={cn({
									"opacity-0 group-data-[selected=true]:opacity-100":
										!isSelected,
								})}
							/>
							<span className="text-accent-foreground">
								{option.label}
							</span>
						</CommandItem>
					);
				})}
			</CommandGroup>
		);
	}
	if (
		definition.type === "select" &&
		typeof definition.options === "function"
	) {
		const Options = definition.options;
		return (
			<Suspense>
				<Options value={value} onSelect={onOptionSelect} />
			</Suspense>
		);
	}

	if (definition.type === "date") {
		return <FilterDateOption onOptionSelect={onOptionSelect} />;
	}
	return null;
}

function FilterDateOption({
	value,
	operator,
	onOptionSelect,
}: {
	value?: string[];
	operator?: FilterOp;
	onOptionSelect: OnOptionSelect;
}) {
	if (operator === FilterOp.BETWEEN) {
		return (
			<FilterDateRange value={value} onOptionSelect={onOptionSelect} />
		);
	}

	return (
		<FilterDateSingle value={value?.[0]} onOptionSelect={onOptionSelect} />
	);
}

function FilterDateSingle({
	value,
	onOptionSelect,
}: { value?: string; onOptionSelect: OnOptionSelect }) {
	const [date, setDate] = useState<Date | undefined>(
		value ? new Date(+value) : undefined,
	);
	const handleDateChange = (date: Date | undefined) => {
		setDate(date);
		if (!date) {
			return;
		}
		if (timeRef.current) {
			const toTime = timeRef.current.value;
			const [hours, minutes, seconds] = parseTime(toTime) || [0, 0, 0];
			date.setHours(hours, minutes, seconds);
		}
		onOptionSelect([String(date.getTime())]);
	};

	const handleTimeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
		const parsedTime = parseTime(e.target.value);
		if (!parsedTime) {
			return;
		}
		const [hours, minutes, seconds] = parsedTime;
		const newDate = new Date(date || Date.now());
		newDate.setHours(hours, minutes, seconds);
		setDate(newDate);
		onOptionSelect([String(newDate.getTime())]);
	};

	const timeRef = useRef<HTMLInputElement>(null);
	return (
		<CommandGroup>
			<CommandItem
				asChild
				value="date"
				className="p-0 aria-selected:bg-transparent flex-col items-start"
			>
				<div>
					<Calendar
						initialFocus
						mode="single"
						defaultMonth={date}
						selected={date}
						onSelect={handleDateChange}
						numberOfMonths={1}
					/>

					<div className="flex gap-2 items-center pb-3 px-3">
						<div className="flex gap-1 flex-col items-start justify-center">
							<Label className="text-sm">Time</Label>
							<Input
								ref={timeRef}
								onChange={handleTimeChange}
								type="time"
								step="1"
								defaultValue={lightFormat(
									date || Date.now(),
									"HH:mm:ss",
								)}
								className="px-2 py-1 h-auto text-sm [&::-webkit-calendar-picker-indicator]:hidden"
							/>
						</div>
					</div>
				</div>
			</CommandItem>
		</CommandGroup>
	);
}

function FilterDateRange({
	value,
	onOptionSelect,
}: {
	value?: string[];
	onOptionSelect: OnOptionSelect;
}) {
	const [date, setDate] = useState<DateRange | undefined>(
		value
			? { from: new Date(+value[0]), to: new Date(+value[1]) }
			: undefined,
	);

	const fromTimeRef = useRef<HTMLInputElement>(null);
	const toTimeRef = useRef<HTMLInputElement>(null);

	const handleDateRangeChange = (date: DateRange | undefined) => {
		setDate(date);

		if (date?.from && date?.to) {
			if (fromTimeRef.current) {
				const fromTime = fromTimeRef.current.value;
				const [hours, minutes, seconds] = parseTime(fromTime) || [
					0, 0, 0,
				];
				date.from.setHours(hours, minutes, seconds);
			}
			if (toTimeRef.current) {
				const toTime = toTimeRef.current.value;
				const [hours, minutes, seconds] = parseTime(toTime) || [
					0, 0, 0,
				];
				date.to.setHours(hours, minutes, seconds);
			}

			onOptionSelect([
				String(date.from.getTime()),
				String(date.to.getTime()),
			]);
		}
	};

	const handleTimeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
		const parsedTime = parseTime(e.target.value);
		if (!parsedTime) {
			return;
		}
		const target = e.target.dataset.target;
		if (!target) {
			return;
		}

		const targetDate = target === "from" ? date?.from : date?.to;

		const [hours, minutes, seconds] = parsedTime;
		const newDate = new Date(targetDate || Date.now());
		newDate.setHours(hours, minutes, seconds);

		setDate((prev) => {
			if (prev?.from && prev?.to) {
				return {
					...prev,
					[target === "from" ? "from" : "to"]: newDate,
				};
			}
			return prev;
		});
		onOptionSelect([
			target === "from"
				? String(newDate.getTime())
				: String(date?.from?.getTime() || Date.now()),
			target === "to"
				? String(newDate.getTime())
				: String(date?.to?.getTime() || Date.now()),
		]);
	};

	return (
		<CommandGroup>
			<Calendar
				initialFocus
				mode="range"
				defaultMonth={date?.from}
				selected={date}
				onSelect={handleDateRangeChange}
				numberOfMonths={1}
			/>
			<div className="flex gap-2 items-center pb-3 px-3">
				<div className="flex gap-1 flex-col items-start justify-center">
					<Label className="text-sm">From</Label>
					<Input
						data-target="from"
						ref={fromTimeRef}
						onChange={handleTimeChange}
						type="time"
						step="1"
						defaultValue={lightFormat(
							date?.from || Date.now(),
							"HH:mm:ss",
						)}
						className="px-2 py-1 h-auto text-sm [&::-webkit-calendar-picker-indicator]:hidden"
					/>
				</div>
				<div className="flex gap-1 flex-col items-start justify-center">
					<Label className="text-sm">To</Label>
					<Input
						data-target="to"
						ref={toTimeRef}
						onChange={handleTimeChange}
						type="time"
						step="1"
						defaultValue={lightFormat(
							date?.to || Date.now(),
							"HH:mm:ss",
						)}
						className="px-2 py-1 h-auto text-sm [&::-webkit-calendar-picker-indicator]:hidden"
					/>
				</div>
			</div>
		</CommandGroup>
	);
}

function parseTime(time: string) {
	const [hours, minutes, seconds] = time
		.split(":")
		.map((val) => Number.parseInt(val) || 0);

	if (Number.isNaN(hours) || Number.isNaN(minutes) || Number.isNaN(seconds)) {
		return null;
	}

	return [hours, minutes, seconds];
}

export type OptionsProviderProps = {
	onSelect: OnOptionSelect;
	value: string[];
};

export type PreviewProviderProps = {
	value: string[];
};

export const FilterValueSchema = z
	.object({
		operator: z.nativeEnum(FilterOp),
		value: z.array(z.string()),
	})
	.optional();

export type FilterValue = z.infer<typeof FilterValueSchema>;

export function createFiltersSchema(definitions: FilterDefinitions) {
	const filters: Record<string, z.ZodTypeAny> = {};
	for (const [key, definition] of Object.entries(definitions)) {
		filters[key] = FilterValueSchema;
	}

	return z.object(filters);
}

export function createFiltersPicker(definitions: FilterDefinitions) {
	return (object: Record<string, unknown>) => {
		const keys = Object.keys(definitions);
		return _.pick(object, keys);
	};
}
