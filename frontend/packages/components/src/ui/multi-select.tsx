import { faCheck, faChevronDown, faCircleX, faX, Icon } from "@rivet-gg/icons";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "../lib/utils";
import { Badge } from "./badge";
import { Button } from "./button";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	CommandSeparator,
} from "./command";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";
import { Separator } from "./separator";

const multiSelectVariants = cva(
	"m-1 transition ease-in-out delay-150 duration-300",
	{
		variants: {
			variant: {
				default:
					"border-foreground/10 drop-shadow-md text-foreground bg-card hover:bg-card/80",
				secondary:
					"border-foreground/10 bg-secondary text-secondary-foreground hover:bg-secondary/80",
				destructive:
					"border-transparent bg-destructive text-destructive-foreground hover:bg-destructive/80",
				inverted: "inverted",
			},
		},
		defaultVariants: {
			variant: "default",
		},
	},
);

interface MultiSelectFormFieldProps
	extends React.ButtonHTMLAttributes<HTMLButtonElement>,
		VariantProps<typeof multiSelectVariants> {
	asChild?: boolean;
	options: {
		label: string;
		value: string;
		icon?: React.ComponentType<{ className?: string }>;
	}[];
	defaultValue?: string[];
	disabled?: boolean;
	placeholder: string;
	className?: string;
	onValueChange: (value: string[]) => void;
}

const MultiSelectFormField = React.forwardRef<
	HTMLButtonElement,
	MultiSelectFormFieldProps
>(
	(
		{
			className,
			variant,
			asChild = false,
			options,
			defaultValue,
			onValueChange,
			disabled,
			placeholder,
			...props
		},
		ref,
	) => {
		const [selectedValues, setSelectedValues] = React.useState<string[]>(
			defaultValue || [],
		);
		const selectedValuesSet = React.useRef(new Set(selectedValues));
		const [isPopoverOpen, setIsPopoverOpen] = React.useState(false);

		React.useEffect(() => {
			setSelectedValues(defaultValue || []);
			selectedValuesSet.current = new Set(defaultValue);
		}, [defaultValue]);

		const handleInputKeyDown: React.KeyboardEventHandler<
			HTMLInputElement
		> = (event) => {
			if (event.key === "Enter") {
				setIsPopoverOpen(true);
			} else if (
				event.key === "Backspace" &&
				!event.currentTarget.value
			) {
				selectedValues.pop();
				setSelectedValues([...selectedValues]);
				selectedValuesSet.current.delete(
					selectedValues[selectedValues.length - 1],
				);
				onValueChange([...selectedValues]);
			}
		};

		const toggleOption = (value: string) => {
			if (selectedValuesSet.current.has(value)) {
				selectedValuesSet.current.delete(value);
				setSelectedValues(selectedValues.filter((v) => v !== value));
			} else {
				selectedValuesSet.current.add(value);
				setSelectedValues([...selectedValues, value]);
			}
			onValueChange(Array.from(selectedValuesSet.current));
		};

		return (
			<Popover open={isPopoverOpen} onOpenChange={setIsPopoverOpen}>
				<PopoverTrigger asChild>
					<Button
						ref={ref}
						{...props}
						disabled={disabled}
						onClick={() => setIsPopoverOpen(!isPopoverOpen)}
						className="flex w-full p-1 rounded-md border min-h-10 h-auto items-center justify-between bg-inherit hover:bg-card"
					>
						{selectedValues.length > 0 ? (
							<div className="flex justify-between items-center w-full">
								<div className="flex flex-wrap items-center">
									{selectedValues.map((value) => {
										const option = options.find(
											(o) => o.value === value,
										);
										const IconComponent = option?.icon;
										return (
											<Badge
												key={value}
												className={cn(
													multiSelectVariants({
														variant,
														className,
													}),
												)}
											>
												{IconComponent && (
													<IconComponent className="h-4 w-4 mr-2" />
												)}
												{option?.label}

												<Icon
													icon={faCircleX}
													className="ml-2 h-4 w-4 cursor-pointer"
													onClick={(event) => {
														event.stopPropagation();
														toggleOption(value);
													}}
												/>
											</Badge>
										);
									})}
								</div>
								<div className="flex items-center justify-between">
									<Icon
										icon={faX}
										className="h-4 mx-2 cursor-pointer text-muted-foreground"
										onClick={(event) => {
											setSelectedValues([]);
											selectedValuesSet.current.clear();
											onValueChange([]);
											event.stopPropagation();
										}}
									/>
									<Separator
										orientation="vertical"
										className="flex min-h-6 h-full"
									/>
									<Icon
										icon={faChevronDown}
										className="h-4 mx-2 cursor-pointer text-muted-foreground"
									/>
								</div>
							</div>
						) : (
							<div className="flex items-center justify-between w-full mx-auto">
								<span className="text-sm text-muted-foreground mx-3">
									{placeholder}
								</span>

								<Icon
									icon={faChevronDown}
									className="h-4 cursor-pointer text-muted-foreground mx-2"
								/>
							</div>
						)}
					</Button>
				</PopoverTrigger>
				<PopoverContent
					className="w-[200px] p-0 drop-shadow-sm"
					align="start"
					onEscapeKeyDown={() => setIsPopoverOpen(false)}
				>
					<Command>
						<CommandInput
							placeholder="Search..."
							onKeyDown={handleInputKeyDown}
						/>
						<CommandList>
							<CommandEmpty>No results found.</CommandEmpty>
							<CommandGroup>
								{options.map((option) => {
									const isSelected =
										selectedValuesSet.current.has(
											option.value,
										);
									return (
										<CommandItem
											key={option.value}
											onSelect={() =>
												toggleOption(option.value)
											}
											style={{
												pointerEvents: "auto",
												opacity: 1,
											}}
											className="cursor-pointer"
										>
											<div
												className={cn(
													"mr-2 flex h-4 w-4 items-center justify-center rounded-sm border border-primary",
													isSelected
														? "bg-primary text-primary-foreground"
														: "opacity-50 [&_svg]:invisible",
												)}
											>
												<Icon
													icon={faCheck}
													className="h-4 w-4"
												/>
											</div>
											{option.icon && (
												<option.icon className="mr-2 h-4 w-4 text-muted-foreground" />
											)}
											<span>{option.label}</span>
										</CommandItem>
									);
								})}
							</CommandGroup>
							<CommandSeparator />
							<CommandGroup>
								<div className="flex items-center justify-between">
									{selectedValues.length > 0 && (
										<>
											<CommandItem
												onSelect={() => {
													setSelectedValues([]);
													selectedValuesSet.current.clear();
													onValueChange([]);
												}}
												style={{
													pointerEvents: "auto",
													opacity: 1,
												}}
												className="flex-1 justify-center cursor-pointer"
											>
												Clear
											</CommandItem>
											<Separator
												orientation="vertical"
												className="flex min-h-6 h-full"
											/>
										</>
									)}
									<CommandSeparator />
									<CommandItem
										onSelect={() => setIsPopoverOpen(false)}
										style={{
											pointerEvents: "auto",
											opacity: 1,
										}}
										className="flex-1 justify-center cursor-pointer"
									>
										Close
									</CommandItem>
								</div>
							</CommandGroup>
						</CommandList>
					</Command>
				</PopoverContent>
			</Popover>
		);
	},
);

MultiSelectFormField.displayName = "MultiSelectFormField";

export { MultiSelectFormField };
