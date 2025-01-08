import * as React from "react";

import { faCheck, faChevronDown } from "@rivet-gg/icons";
import { Icon } from "@rivet-gg/icons";
import { Fragment } from "react";
import { cn } from "../lib/utils";
import { Button } from "./button";
import { Command, CommandInput, CommandItem, CommandList } from "./command";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";

export interface Option {
	label: React.ReactNode;
	value: string;
}

interface ComboboxNewOptionsProps {
	allowCreate: true;
	onCreateOption: (option: string) => void;
}

interface ComboboxDefaultProps {
	allowCreate?: false;
	onCreateOption?: never;
}

interface ComboboxSingleProps {
	multiple?: false;
	value: string;
	onValueChange: (value: string) => void;
}

interface ComboboxMultipleProps {
	multiple: true;
	value: string[];
	onValueChange: (value: string[]) => void;
}

export type ComboboxProps = {
	options: Option[];
	placeholder?: string;
	notFoundMessage?: string;
	className?: string;
} & (ComboboxNewOptionsProps | ComboboxDefaultProps) &
	(ComboboxSingleProps | ComboboxMultipleProps);

export const Combobox = React.forwardRef<HTMLButtonElement, ComboboxProps>(
	(
		{
			options,
			placeholder,
			notFoundMessage,
			className,
			value,
			multiple,
			onValueChange,
			...props
		},
		ref,
	) => {
		const [search, setSearch] = React.useState("");
		const [open, onOpenChange] = React.useState(false);

		const filteredOptions = options.filter((option) =>
			option.value.toLowerCase().includes(search.toLowerCase()),
		);

		const handleSelect = (newValue: string) => {
			if (multiple) {
				const newValues = Array.isArray(value) ? value : [];
				if (newValues.includes(newValue)) {
					onValueChange(newValues.filter((v) => v !== newValue));
				} else {
					onValueChange([...newValues, newValue]);
				}
			} else {
				onValueChange(newValue);
			}
			onOpenChange(false);
		};

		const handleNewSelect = (value: string) => {
			if (props.allowCreate) {
				React.startTransition(() => {
					handleSelect(value);
					props.onCreateOption(value);
				});
			}
		};

		const currentOptions = options.filter((option) =>
			Array.isArray(value)
				? value.includes(option.value)
				: option.value === value,
		);

		return (
			<Popover open={open} onOpenChange={onOpenChange}>
				<PopoverTrigger asChild>
					<Button
						ref={ref}
						variant="outline"
						// biome-ignore lint/a11y/useSemanticElements: combobox is a custom component
						role="combobox"
						aria-expanded={open}
						className={cn(
							"justify-between",
							currentOptions.length === 0 &&
								"text-muted-foreground/50",
							className,
						)}
					>
						{currentOptions.length > 0
							? currentOptions.map((option) => (
									<Fragment key={option.value}>
										{option.label}
									</Fragment>
								))
							: placeholder}

						<Icon
							className="ml-2 h-4 w-4 shrink-0 text-foreground opacity-50"
							icon={faChevronDown}
						/>
					</Button>
				</PopoverTrigger>
				<PopoverContent className="p-0 w-[--radix-popover-trigger-width]">
					<Command shouldFilter={false} loop>
						<CommandInput
							value={search}
							onValueChange={setSearch}
							placeholder={placeholder}
						/>
						<CommandList>
							{filteredOptions.map((option) => {
								return (
									<ComboboxOption
										key={option.value}
										isCurrent={
											Array.isArray(value)
												? value.includes(option.value)
												: value === option.value
										}
										label={option.label}
										value={option.value}
										onSelect={handleSelect}
									/>
								);
							})}
							{filteredOptions.length === 0 ? (
								<ComboboxOption
									label={search}
									value={search}
									onSelect={handleNewSelect}
								/>
							) : null}
						</CommandList>
					</Command>
				</PopoverContent>
			</Popover>
		);
	},
);

interface ComboboxOptionProps {
	isCurrent?: boolean;
	label: Option["label"];
	value: Option["value"];
	onSelect: (value: string) => void;
}

function ComboboxOption({
	isCurrent,
	label,
	value,
	onSelect,
}: ComboboxOptionProps) {
	return (
		<CommandItem
			key={value}
			value={value}
			keywords={[value]}
			onSelect={onSelect}
		>
			<Icon
				icon={faCheck}
				className={cn(
					"mr-2 h-4 w-4",
					isCurrent ? "opacity-100" : "opacity-0",
				)}
			/>
			{label}
		</CommandItem>
	);
}
