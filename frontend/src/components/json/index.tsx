import { Slot, Slottable } from "@radix-ui/react-slot";
import {
	faCaretDown,
	faCaretRight,
	faCopy,
	faPencil,
	faTrash,
	Icon,
} from "@rivet-gg/icons";
import type { OnChangeFn } from "@tanstack/react-table";
import _ from "lodash";
import {
	createContext,
	type Dispatch,
	type ReactElement,
	type SetStateAction,
	useContext,
	useEffect,
	useLayoutEffect,
	useRef,
	useState,
} from "react";
import { CopyButton } from "../copy-area";
import { Checkbox } from "../ui/checkbox";
import { WithTooltip } from "../ui/tooltip";

type SetState<T> = Dispatch<SetStateAction<T>>;

const OpenContext = createContext<Record<string, boolean>>({});
const SetOpenContext = createContext<SetState<Record<string, boolean>>>(
	(prev) => prev,
);
const ChangeValueContext = createContext<OnChangeFn<Record<string, unknown>>>(
	(prev) => prev,
);

export function Json({
	value,
	onChange,
}: {
	value: unknown;
	onChange: OnChangeFn<Record<string, unknown>>;
}) {
	const [open, setOpen] = useState<Record<string, boolean>>({ $: true });
	return (
		<div
			className="font-mono-console flex flex-col w-full text-xs p-2 w-full"
			role="tree"
		>
			<ChangeValueContext.Provider value={onChange}>
				<SetOpenContext.Provider value={setOpen}>
					<OpenContext.Provider value={open}>
						<Value path="$" value={value} level={0} />
					</OpenContext.Provider>
				</SetOpenContext.Provider>
			</ChangeValueContext.Provider>
		</div>
	);
}

function Value({
	value,
	path,
	object,
	array,
	level,
	editable = true,
	...props
}: {
	value: unknown;
	path: string;
	editable?: boolean;
	level?: number;
	object?: (props: {
		value: Record<string, unknown>;
		path: string;
	}) => ReactElement;
	array?: (props: { value: unknown[]; path: string }) => ReactElement;
}) {
	if (typeof value === "string") {
		return (
			<StringValue
				path={path}
				value={value}
				editable={editable}
				{...props}
			/>
		);
	}
	if (typeof value === "number") {
		return (
			<NumberValue
				path={path}
				value={value}
				editable={editable}
				{...props}
			/>
		);
	}
	if (typeof value === "boolean") {
		return (
			<BooleanValue
				path={path}
				value={value}
				editable={editable}
				{...props}
			/>
		);
	}
	if (value === null) {
		return <NullValue path={path} editable={editable} {...props} />;
	}
	if (value === undefined) {
		return <UndefinedValue path={path} editable={editable} {...props} />;
	}
	if (isObject(value)) {
		const Comp = object ?? ObjectValue;
		return <Comp path={path} value={value} level={level} {...props} />;
	}
	if (isArray(value)) {
		const Comp = array ?? ObjectValue;
		return <Comp path={path} value={value} level={level} {...props} />;
	}
}

function isObject(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isArray(value: unknown): value is unknown[] {
	return Array.isArray(value);
}

function hasChildren(value: unknown): boolean {
	if (isObject(value)) {
		return Object.keys(value).length > 0;
	}
	if (isArray(value)) {
		return value.length > 0;
	}
	return false;
}

function StringValue({
	value,
	path,
	editable,
	...props
}: {
	value: string;
	path: string;
	editable?: boolean;
}) {
	const changeValue = useContext(ChangeValueContext);

	const preview = <span>{JSON.stringify(value)}</span>;

	return (
		<div className="text-primary" {...props}>
			{editable ? (
				<Editable
					value={JSON.stringify(value)}
					path={path}
					onChange={(newValue) => {
						const parsedValue = isSafeJson(newValue)
							? JSON.parse(newValue)
							: newValue;

						changeValue((prev: Record<string, unknown>) => {
							_.set(prev, path.slice(2), parsedValue);
							return prev;
						});
					}}
					onDelete={() => {
						changeValue((prev: Record<string, unknown>) => {
							_.unset(prev, path.slice(2));
							return prev;
						});
					}}
				>
					{preview}
				</Editable>
			) : (
				preview
			)}
		</div>
	);
}

function NumberValue({
	value,
	path,
	editable,
	...props
}: {
	path: string;
	value: number;
	editable?: boolean;
}) {
	const changeValue = useContext(ChangeValueContext);

	const preview = <span>{value}</span>;

	return (
		<div {...props}>
			{editable ? (
				<Editable
					value={value.toString()}
					path={path}
					onChange={(newValue) => {
						const num = Number.parseFloat(newValue);
						const parsedValue = !Number.isNaN(num)
							? num
							: isSafeJson(newValue)
								? JSON.parse(newValue)
								: newValue;

						changeValue((prev: Record<string, unknown>) => {
							const existingValue = _.get(prev, path.slice(2));
							if (existingValue === parsedValue) {
								return prev;
							}
							_.set(prev, path.slice(2), parsedValue);
							return prev;
						});
					}}
					onDelete={() => {
						changeValue((prev: Record<string, unknown>) => {
							_.unset(prev, path.slice(2));
							return prev;
						});
					}}
				>
					{preview}
				</Editable>
			) : (
				preview
			)}
		</div>
	);
}

function BooleanValue({
	value,
	path,
	editable,
	...props
}: {
	value: boolean;
	path: string;
	editable?: boolean;
}) {
	const changeValue = useContext(ChangeValueContext);
	const preview = <span>{JSON.stringify(value)}</span>;

	return (
		<div className="flex items-center gap-1" {...props}>
			{editable ? (
				<Editable
					value={JSON.stringify(value)}
					path={path}
					onChange={(newValue) => {
						const parsedValue = isSafeJson(newValue)
							? JSON.parse(newValue)
							: newValue;

						changeValue((prev: Record<string, unknown>) => {
							_.set(prev, path.slice(2), parsedValue);
							return prev;
						});
					}}
					onDelete={() => {
						changeValue((prev: Record<string, unknown>) => {
							_.unset(prev, path.slice(2));
							return prev;
						});
					}}
				>
					<div className="flex items-center gap-1">
						<Checkbox
							checked={value}
							onCheckedChange={(isChecked) => {
								changeValue((prev: Record<string, unknown>) => {
									_.set(prev, path.slice(2), isChecked);
									return prev;
								});
							}}
							className="h-4 w-4"
						/>

						<span>{JSON.stringify(value)}</span>
					</div>
				</Editable>
			) : (
				preview
			)}
		</div>
	);
}

function NullValue({
	path,
	editable,
	...props
}: {
	path: string;
	editable?: boolean;
}) {
	const changeValue = useContext(ChangeValueContext);
	const preview = <span className="text-foreground/80">null</span>;

	return (
		<div {...props}>
			{editable ? (
				<Editable
					value={JSON.stringify(null)}
					path={path}
					onChange={(newValue) => {
						const parsedValue = isSafeJson(newValue)
							? JSON.parse(newValue)
							: newValue;

						changeValue((prev: Record<string, unknown>) => {
							_.set(prev, path.slice(2), parsedValue);
							return prev;
						});
					}}
					onDelete={() => {
						changeValue((prev: Record<string, unknown>) => {
							_.unset(prev, path.slice(2));
							return prev;
						});
					}}
				>
					{preview}
				</Editable>
			) : (
				preview
			)}
		</div>
	);
}

function UndefinedValue({
	editable,
	path,
	...props
}: {
	path: string;
	editable?: boolean;
}) {
	const changeValue = useContext(ChangeValueContext);

	const preview = <span>null</span>;
	return (
		<div {...props}>
			{editable ? (
				<Editable
					value={JSON.stringify(null)}
					path={path}
					onChange={(newValue) => {
						const parsedValue = isSafeJson(newValue)
							? JSON.parse(newValue)
							: newValue;

						changeValue((prev: Record<string, unknown>) => {
							_.set(prev, path.slice(2), parsedValue);
							return prev;
						});
					}}
					onDelete={() => {
						changeValue((prev: Record<string, unknown>) => {
							_.unset(prev, path.slice(2));
							return prev;
						});
					}}
				>
					{preview}
				</Editable>
			) : (
				preview
			)}
		</div>
	);
}

function ObjectValue({
	path,
	value,
	level = 0,
}: {
	path: string;
	value: Record<string, unknown> | unknown[];
	level?: number;
}) {
	const isOpen = useContext(OpenContext);
	const setIsOpen = useContext(SetOpenContext);

	const isCurrentOpen = isOpen[path] ?? false;

	const keys = Object.keys(value);

	const handleOpen = (e: React.MouseEvent | React.KeyboardEvent) => {
		e.stopPropagation();
		if (!hasChildren(value)) {
			return;
		}
		if (("key" in e && e.key === " ") || e.type === "click") {
			setIsOpen((prev) => ({
				...prev,
				[path]: !prev[path],
			}));
		}
	};

	return (
		<div className="flex flex-col w-full">
			<div className="flex items-center gap-2 w-full">
				{!isCurrentOpen ? (
					<div
						className="flex cursor-pointer opacity-90"
						onClick={handleOpen}
						onKeyDown={handleOpen}
						role="treeitem"
						aria-expanded={isCurrentOpen}
						tabIndex={0}
					>
						<span className="mr-2">{`{`}</span>
						{keys.slice(0, 4).map((key, index, array) => {
							const isLast = index === array.length - 1;
							return (
								<div key={key} className="flex italic">
									<span className="text-foreground/70 mr-1">
										{key}:
									</span>{" "}
									<Value
										value={value[key]}
										path={`${path}.${key}`}
										editable={false}
										object={() => (
											<span>
												{"{"}...{"}"}
											</span>
										)}
										array={({ value }) => (
											<span>
												Array(
												{value.length})
											</span>
										)}
									/>
									{!isLast ? (
										<span className="text-muted-foreground mr-2">
											,{" "}
										</span>
									) : null}
								</div>
							);
						})}
						{keys.length > 4 ? (
							<div className="text-muted-foreground ml-2">
								<span>…</span>
							</div>
						) : null}
						<span className="ml-2">{`}`}</span>
					</div>
				) : null}
			</div>
			{isCurrentOpen && (
				<div
					onClick={handleOpen}
					onKeyDown={handleOpen}
					role="treeitem"
					aria-expanded={isCurrentOpen}
					tabIndex={0}
					className="flex flex-col gap-1 transition-all"
				>
					{keys.map((key) => {
						const childrenPath = `${path}.${key}`;
						return (
							<Property
								level={level}
								editablePropertyName={!isArray(value)}
								key={key}
								value={value[key]}
								property={key}
								path={childrenPath}
							/>
						);
					})}
				</div>
			)}
		</div>
	);
}

function Property({
	value,
	level = 0,
	property,
	editablePropertyName = true,
	path,
}: {
	value: unknown;
	path: string;
	property: string;
	level?: number;
	editablePropertyName?: boolean;
}) {
	const isOpen = useContext(OpenContext);
	const setIsOpen = useContext(SetOpenContext);
	const changeValue = useContext(ChangeValueContext);
	const handleOpen = (e: React.MouseEvent | React.KeyboardEvent) => {
		e.stopPropagation();
		if (!hasChildren(value)) {
			return;
		}
		if (("key" in e && e.key === " ") || e.type === "click") {
			e.preventDefault();
			setIsOpen((prev) => ({
				...prev,
				[path]: !prev[path],
			}));
		}
	};

	const propertyNamePreview = <span>{property}</span>;

	return (
		<div
			data-slot="property"
			tabIndex={0}
			role="treeitem"
			data-level={level}
			aria-expanded={isOpen[path] ?? false}
			className="flex pl-4 [&[data-level='0']]:pl-0 hover:bg-muted-foreground/10 [&>div>div>div>div>[data-action]]:opacity-0 [&:hover>div>div>div>div>[data-action]]:opacity-100 has-[[data-slot='property']:hover]:bg-transparent rounded items-start transition-colors"
			onKeyDown={handleOpen}
			onClick={handleOpen}
		>
			<div
				data-open={isOpen[path] ?? false}
				className="flex items-start [&[data-open='true']]:flex-col"
			>
				<div className="flex items-center">
					{hasChildren(value) ? (
						<Icon
							className="size-3 mr-1 opacity-70 transition-opacity"
							icon={isOpen[path] ? faCaretDown : faCaretRight}
						/>
					) : (
						<div className="size-3 mr-1" />
					)}
					<span className="flex mr-1">
						{editablePropertyName ? (
							<Editable
								value={property}
								className="cursor-pointer"
								showIcon={false}
								path={path}
								onChange={(newKey) => {
									const newPath = `${path.slice(
										0,
										-property.length - 1,
									)}.${newKey}`;
									setIsOpen((prev) => ({
										...prev,
										[path]: false,
										[newPath]: prev[path],
									}));

									changeValue(
										(prev: Record<string, unknown>) => {
											const value = _.get(
												prev,
												// removes $. from the start
												path.slice(2),
											);
											_.unset(
												prev,
												// removes $. from the start
												path.slice(2),
											);

											return _.set(
												prev,
												// removes $. from the start
												newPath.slice(2),
												value,
											);
										},
									);
								}}
								onDelete={() => {
									changeValue(
										(prev: Record<string, unknown>) => {
											_.unset(
												prev,
												// removes $. from the start
												path.slice(2),
											);
											return prev;
										},
									);
								}}
							>
								{propertyNamePreview}
							</Editable>
						) : (
							propertyNamePreview
						)}
						:
					</span>
				</div>
				<Value value={value} path={path} level={level + 1} />
			</div>
		</div>
	);
}

function Editable({
	value,
	path,
	children,
	showIcon = true,
	as = "input",
	onChange,
	onDelete,
}: {
	showIcon?: boolean;
	value: string;
	children: React.ReactNode;
	path: string;
	as?: "input" | "textarea";
	onChange?: (newValue: string) => void;
	onDelete?: () => void;
}) {
	const [isEditing, setIsEditing] = useState(false);
	const ref = useRef<HTMLInputElement | HTMLTextAreaElement>(null);

	function edit() {
		const alreadyEditing = ref.current?.querySelector(
			'[data-editing="true"]',
		);
		if (!alreadyEditing) {
			setIsEditing(true);
		}
	}

	useClickOutside(ref, () => {
		if (isEditing) {
			onChange?.(ref.current?.value ?? "");
			setIsEditing(false);
		}
	});

	useLayoutEffect(() => {
		if (isEditing && ref.current) {
			ref.current.focus();
			ref.current.select();
		}
	}, [isEditing]);

	if (isEditing) {
		const Comp = as;
		return (
			<Comp
				ref={ref}
				data-editing="true"
				type="text"
				onClick={(e) => {
					e.stopPropagation();
				}}
				onKeyDown={(e) => {
					e.stopPropagation();
					if (e.key === "Enter") {
						onChange?.(ref.current?.value ?? "");
						setIsEditing(false);
					}
				}}
				name={path}
				defaultValue={value}
				className="bg-transparent field-sizing-content font-mono-console text-foreground outline-none focus:ring-0 w-full focus:border-0 focus:ring focus:ring-1 focus:ring-primary/40 rounded-sm border-primary selection-primary"
			/>
		);
	}

	return (
		<div className="flex items-center gap-2">
			<WithTooltip
				content="Double click to edit"
				trigger={
					<Slot
						className="flex items-center cursor-pointer"
						ref={ref}
						onDoubleClick={(e) => {
							e.stopPropagation();
							edit();
						}}
						onKeyDown={(e) => {
							if (e.key === "Enter") {
								edit();
							}
						}}
						tabIndex={0}
					>
						<Slottable>{children}</Slottable>
					</Slot>
				}
			/>

			{showIcon ? (
				<div className="flex gap-1.5 cursor-pointer">
					<WithTooltip
						content="Edit"
						trigger={
							<Icon
								data-action="edit"
								onClick={(e) => {
									e.stopPropagation();
									edit();
								}}
								className="text-foreground transition-opacity size-2.5 opacity-70 hover:opacity-100"
								icon={faPencil}
							/>
						}
					/>

					<WithTooltip
						content="Copy"
						trigger={
							<CopyButton value={value}>
								<Icon
									data-action="copy"
									className="text-foreground transition-opacity size-2.5 opacity-70 hover:opacity-100"
									icon={faCopy}
								/>
							</CopyButton>
						}
					/>

					<WithTooltip
						content="Delete"
						trigger={
							<Icon
								data-action="delete"
								onClick={(e) => {
									e.stopPropagation();
									onDelete?.();
								}}
								className="text-foreground transition-opacity size-2.5 opacity-70 hover:opacity-100"
								icon={faTrash}
							/>
						}
					/>
				</div>
			) : null}
		</div>
	);
}

function useClickOutside(
	ref: React.RefObject<HTMLElement>,
	callback: () => void,
) {
	const callbackRef = useRef(callback);

	useEffect(() => {
		callbackRef.current = callback;
	});

	useEffect(() => {
		const handleClickOutside = (event: MouseEvent) => {
			if (ref.current && !ref.current.contains(event.target as Node)) {
				callbackRef.current();
			}
		};

		document.addEventListener("mousedown", handleClickOutside);
		return () => {
			document.removeEventListener("mousedown", handleClickOutside);
		};
	}, [ref]);
}

function isSafeJson(json: string): boolean {
	try {
		JSON.parse(json);
		return true;
	} catch (e) {
		return false;
	}
}
