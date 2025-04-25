"use client";
import { Checkbox } from "./checkbox";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	CommandSeparator,
} from "./command";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "./dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";
import { nanoid } from "nanoid";
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
} from "react";
import { Button } from "./button";
import { motion } from "framer-motion";
import { Badge } from "./badge";

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

export enum FilterOperator {
	IS = "is",
	IS_NOT = "is not",
	IS_ANY_OF = "is any of",
	INCLUDE = "include",
	DO_NOT_INCLUDE = "do not include",
	INCLUDE_ALL_OF = "include all of",
	INCLUDE_ANY_OF = "include any of",
	EXCLUDE_ALL_OF = "exclude all of",
	EXCLUDE_IF_ANY_OF = "exclude if any of",
	BEFORE = "before",
	AFTER = "after",
}

export type Filter = {
	id: string;
	defId: string;
	operator: FilterOperator;
	value: string[];
};

function filterDefinitionToOptions(definition: FilterDefinition) {
	if (definition.type === "select") {
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
	if (definition.type === "select") {
		if (filterValues.length > 1) {
			return FilterOperator.IS_ANY_OF;
		}
		return FilterOperator.IS;
	}
	return FilterOperator.IS;
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
			if (Array.isArray(filterValues) && filterValues.length > 1) {
				return [FilterOperator.IS_ANY_OF, FilterOperator.IS_NOT];
			}
			return [FilterOperator.IS, FilterOperator.IS_NOT];
		// case FilterType.STATUS:
		// case FilterType.ASSIGNEE:
		// case FilterType.PRIORITY:
		// 	if (Array.isArray(filterValues) && filterValues.length > 1) {
		// 		return [FilterOperator.IS_ANY_OF, FilterOperator.IS_NOT];
		// 	} else {
		// 		return [FilterOperator.IS, FilterOperator.IS_NOT];
		// 	}
		// case FilterType.LABELS:
		// 	if (Array.isArray(filterValues) && filterValues.length > 1) {
		// 		return [
		// 			FilterOperator.INCLUDE_ANY_OF,
		// 			FilterOperator.INCLUDE_ALL_OF,
		// 			FilterOperator.EXCLUDE_ALL_OF,
		// 			FilterOperator.EXCLUDE_IF_ANY_OF,
		// 		];
		// 	} else {
		// 		return [FilterOperator.INCLUDE, FilterOperator.DO_NOT_INCLUDE];
		// 	}
		// case FilterType.DUE_DATE:
		// case FilterType.CREATED_DATE:
		// case FilterType.UPDATED_DATE:
		// 	if (filterValues?.includes(DueDate.IN_THE_PAST)) {
		// 		return [FilterOperator.IS, FilterOperator.IS_NOT];
		// 	} else {
		// 		return [FilterOperator.BEFORE, FilterOperator.AFTER];
		// 	}
		default:
			return [];
	}
};

const FilterOperatorDropdown = ({
	definition,
	operator,
	filterValues,
	setOperator,
}: {
	definition: FilterDefinition;
	operator: FilterOperator;
	filterValues: string[];
	setOperator: (operator: FilterOperator) => void;
}) => {
	const operators = filterOperators({ definition, filterValues });
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="outline"
					size="xs"
					className="shrink-0 rounded-none px-1 border-y-0 ml-1 mr-0"
				>
					{operator}
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="start" className="w-fit min-w-fit">
				{operators.map((operator) => (
					<DropdownMenuItem
						key={operator}
						onClick={() => setOperator(operator)}
					>
						{operator}
					</DropdownMenuItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
};

const FilterValueCombobox = ({
	definition,
	filterValues,
	setFilterValues,
}: {
	definition: FilterDefinition;
	filterValues: string[];
	setFilterValues: (filterValues: string[]) => void;
}) => {
	const [open, setOpen] = useState(false);
	const [commandInput, setCommandInput] = useState("");
	const commandInputRef = useRef<HTMLInputElement>(null);

	const options = filterDefinitionToOptions(definition);

	const nonSelectedOptions = options.filter(
		(option) => !filterValues.includes(option.value),
	);
	const selectedOptions = options.filter((option) =>
		filterValues.includes(option.value),
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
					className="shrink-0 rounded-none px-1 border-y-0 mx-1 border-l-0 mx-0"
				>
					{selectedOptions?.length === 1
						? selectedOptions?.[0].label
						: `${selectedOptions?.length} selected`}
				</Button>
			</PopoverTrigger>
			<PopoverContent className="w-[200px] p-0">
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
							<CommandGroup>
								{selectedOptions.map((option) => (
									<CommandItem
										key={option.value}
										className="group flex gap-2 items-center"
										onSelect={() => {
											setFilterValues(
												filterValues.filter(
													(v) => v !== option.value,
												),
											);
											setTimeout(() => {
												setCommandInput("");
											}, 200);
											setOpen(false);
										}}
									>
										<Checkbox checked={true} />
										{/* <FilterIcon
											type={value as FilterType}
										/> */}
										{option.label}
									</CommandItem>
								))}
							</CommandGroup>
							{nonSelectedOptions?.length > 0 && (
								<>
									<CommandSeparator />
									<CommandGroup>
										{nonSelectedOptions.map((filter) => (
											<CommandItem
												className="group flex gap-2 items-center"
												key={filter.value}
												value={filter.value}
												onSelect={(currentValue) => {
													setFilterValues([
														...filterValues,
														currentValue,
													]);
													setTimeout(() => {
														setCommandInput("");
													}, 200);
													setOpen(false);
												}}
											>
												<Checkbox
													checked={false}
													className="opacity-0 group-data-[selected=true]:opacity-100"
												/>
												{/* {filter.icon} */}
												<span className="text-accent-foreground">
													{filter.label}
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
	);
};

const FilterValueDateCombobox = ({
	filterType,
	filterValues,
	setFilterValues,
}: {
	filterType: FilterType;
	filterValues: string[];
	setFilterValues: (filterValues: string[]) => void;
}) => {
	const [open, setOpen] = useState(false);
	const [commandInput, setCommandInput] = useState("");
	const commandInputRef = useRef<HTMLInputElement>(null);
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
			<PopoverTrigger
				className="rounded-none px-1.5 py-1 bg-muted hover:bg-muted/50 transition
  text-muted-foreground hover:text-primary shrink-0"
			>
				{filterValues?.[0]}
			</PopoverTrigger>
			<PopoverContent className="w-fit p-0">
				<AnimateChangeInHeight>
					<Command>
						<CommandInput
							placeholder={filterType}
							className="h-9"
							value={commandInput}
							onInputCapture={(e) => {
								setCommandInput(e.currentTarget.value);
							}}
							ref={commandInputRef}
						/>
						<CommandList>
							<CommandEmpty>No results found.</CommandEmpty>
							<CommandGroup>
								{filterDefinitionToOptions.map(
									(filter: FilterOption) => (
										<CommandItem
											className="group flex gap-2 items-center"
											key={filter.name}
											value={filter.name}
											onSelect={(
												currentValue: string,
											) => {
												setFilterValues([currentValue]);
												setTimeout(() => {
													setCommandInput("");
												}, 200);
												setOpen(false);
											}}
										>
											<span className="text-accent-foreground">
												{filter.name}
											</span>
											<Icon
												icon={faCheck}
												className={cn(
													"ml-auto",
													filterValues.includes(
														filter.name,
													)
														? "opacity-100"
														: "opacity-0",
												)}
											/>
										</CommandItem>
									),
								)}
							</CommandGroup>
						</CommandList>
					</Command>
				</AnimateChangeInHeight>
			</PopoverContent>
		</Popover>
	);
};

function FilterValue({
	definition,
	filterValues,
	setFilterValues,
}: {
	definition: FilterDefinition;
	filterValues: string[];
	setFilterValues: (filterValues: string[]) => void;
}) {
	if (definition.type === "select") {
		return (
			<FilterValueCombobox
				definition={definition}
				filterValues={filterValues}
				setFilterValues={setFilterValues}
			/>
		);
	}
}

export default function Filters({
	filters,
	setFilters,
	definitions,
}: {
	filters: Filter[];
	setFilters: Dispatch<SetStateAction<Filter[]>>;
	definitions: FilterDefinition[];
}) {
	return (
		<div className="flex gap-2 flex-wrap">
			{filters
				.filter((filter) => filter.value?.length > 0)
				.map((filter) => {
					const definition = definitions.find(
						(def) => def.id === filter.defId,
					);

					if (!definition) return null;
					return (
						<Badge
							variant="outline"
							key={filter.id}
							className="p-0"
						>
							<Icon
								icon={definition.icon}
								className="mr-1 ml-1.5"
							/>
							<span className="mr-0.5">{definition.label}</span>
							<FilterOperatorDropdown
								definition={definition}
								operator={filter.operator}
								filterValues={filter.value}
								setOperator={(operator) => {
									setFilters((prev) =>
										prev.map((f) =>
											f.id === filter.id
												? { ...f, operator }
												: f,
										),
									);
								}}
							/>
							<FilterValue
								definition={definition}
								filterValues={filter.value}
								setFilterValues={(filterValues) => {
									setFilters((prev) =>
										prev.map((f) => {
											if (f.id === filter.id) {
												const allowedOperators =
													filterOperators({
														definition,
														filterValues,
													});

												if (
													allowedOperators.includes(
														f.operator,
													)
												) {
													return {
														...f,
														value: filterValues,
													};
												}

												// If the operator is not allowed, set it to the first allowed operator
												const newOperator =
													allowedOperators[0];
												return {
													...f,
													operator: newOperator,
													value: filterValues,
												};
											}
											return f;
										}),
									);
								}}
							/>
							<Button
								className="px-0.5 pr-1.5 rounded-none"
								variant="ghost"
								size="xs"
								onClick={() => {
									setFilters((prev) =>
										prev.filter((f) => f.id !== filter.id),
									);
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

export type FilterDefinition = {
	type: "date" | "string" | "number" | "boolean" | "select";
	label: string;
	icon: IconProp;
	id: string;
	options: { value: string; label: ReactNode }[];
};

export const FilterCreator = ({
	definitions,
	filters,
	setFilters,
}: {
	definitions: FilterDefinition[];
	filters: Filter[];
	setFilters: Dispatch<SetStateAction<Filter[]>>;
}) => {
	const [open, setOpen] = useState(false);
	const [selectedDefId, setSelectedDefId] = useState<string | null>(null);
	const [commandInput, setCommandInput] = useState("");
	const commandInputRef = useRef<HTMLInputElement>(null);

	const selectedDefinition = definitions.find(
		(definition) => definition.id === selectedDefId,
	);
	return (
		<div className="flex gap-2 flex-wrap items-center py-1">
			<Filters
				filters={filters}
				setFilters={setFilters}
				definitions={definitions}
			/>
			{filters.filter((filter) => filter.value?.length > 0).length >
				0 && (
				<Button
					variant="outline"
					size="sm"
					onClick={() => setFilters([])}
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
						{!filters.length && "Filter"}
					</Button>
				</PopoverTrigger>
				<PopoverContent className="w-[200px] p-0">
					<AnimateChangeInHeight>
						<Command>
							<CommandInput
								placeholder={
									selectedDefinition
										? selectedDefinition.label
										: "Filter..."
								}
								className="h-9"
								value={commandInput}
								onInputCapture={(e) => {
									setCommandInput(e.currentTarget.value);
								}}
								ref={commandInputRef}
							/>
							<CommandList>
								<CommandEmpty>No results found.</CommandEmpty>
								{selectedDefinition ? (
									<CommandGroup>
										{filterDefinitionToOptions(
											selectedDefinition,
										).map((filter) => (
											<CommandItem
												className="group text-muted-foreground flex gap-2 items-center"
												key={filter.value}
												value={filter.value}
												onSelect={(currentValue) => {
													setFilters((prev) => [
														...prev,
														{
															id: nanoid(),
															defId: selectedDefinition.id,
															operator:
																defaultFilterDefinitionOperator(
																	{
																		definition:
																			selectedDefinition,
																		filterValues:
																			[
																				currentValue,
																			],
																	},
																),
															value: [
																currentValue,
															],
														},
													]);
													setTimeout(() => {
														setSelectedDefId(null);
														setCommandInput("");
													}, 200);
													setOpen(false);
												}}
											>
												{/* {filter.icon} */}
												<span className="text-accent-foreground">
													{filter.label}
												</span>
											</CommandItem>
										))}
									</CommandGroup>
								) : (
									<CommandGroup>
										{definitions.map((definition) => (
											<CommandItem
												key={definition.id}
												className="group text-muted-foreground flex gap-2 items-center"
												value={definition.id}
												onSelect={() => {
													setSelectedDefId(
														definition.id,
													);
													setCommandInput("");
													commandInputRef.current?.focus();
												}}
											>
												<Icon icon={definition.icon} />
												<span className="text-accent-foreground">
													{definition.label}
												</span>
											</CommandItem>
										))}
									</CommandGroup>
								)}
							</CommandList>
						</Command>
					</AnimateChangeInHeight>
				</PopoverContent>
			</Popover>
		</div>
	);
};
