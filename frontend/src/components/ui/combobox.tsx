import { faCheck, faChevronDown, Icon } from "@rivet-gg/icons";
import * as React from "react";
import { Fragment } from "react";
import { cn } from "../lib/utils";
import { VisibilitySensor } from "../visibility-sensor";
import { Badge } from "./badge";
import { Button } from "./button";
import {
	Command,
	CommandInput,
	CommandItem,
	CommandList,
	CommandLoading,
} from "./command";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";

export interface ComboboxOption {
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

export type ComboboxProps<Option extends ComboboxOption> = {
	options: Option[];
	placeholder?: string;
	notFoundMessage?: string;
	className?: string;
	showSelectedOptions?: number;
	filter?: (option: Option, search: string) => boolean;
	onLoadMore?: () => void;
	isLoading?: boolean;
} & (ComboboxNewOptionsProps | ComboboxDefaultProps) &
	(ComboboxSingleProps | ComboboxMultipleProps);

export const Combobox = <Option extends ComboboxOption>({
	options,
	placeholder,
	notFoundMessage,
	className,
	value,
	multiple,
	onValueChange,
	filter,
	showSelectedOptions = 3,
	onLoadMore,
	allowSearchAsOption,
	isLoading,
	...props
}: ComboboxProps<Option>) => {
	const [search, setSearch] = React.useState("");
	const [open, onOpenChange] = React.useState(false);

	const handleSelect = (newValue: string) => {
		React.startTransition(() => {
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
		});
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

	const currentValues = currentOptions.map((opt) => opt.value);

	const filteredOptions = filter
		? options.filter((option) => filter(option, search))
		: options.filter((option) =>
				option.value.toLowerCase().includes(search.toLowerCase()),
			);

	const sorted = filteredOptions.toSorted((a, b) => {
		if (
			currentValues.includes(a.value) &&
			currentValues.includes(b.value)
		) {
			return 0;
		}
		if (currentValues.includes(a.value)) {
			return -1;
		}
		if (currentValues.includes(b.value)) {
			return 1;
		}
		return 0;
	});

	return (
		<Popover open={open} onOpenChange={onOpenChange}>
			<PopoverTrigger asChild>
				<Button
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
					<div className="flex gap-4">
						{currentOptions.length > 0 ? (
							<>
								{currentOptions
									.map((option) => (
										<Fragment key={option.value}>
											{option.label}
										</Fragment>
									))
									.slice(0, showSelectedOptions)}

								{currentOptions.length > showSelectedOptions ? (
									<Badge variant="outline">
										+
										{currentOptions.length -
											showSelectedOptions}
									</Badge>
								) : null}
							</>
						) : (
							placeholder
						)}
					</div>

					<Icon
						className="ml-2 h-4 w-4 shrink-0 text-foreground opacity-50"
						icon={faChevronDown}
					/>
				</Button>
			</PopoverTrigger>
			<PopoverContent
				className="p-0 w-[--radix-popover-trigger-width]"
				// https://github.com/radix-ui/primitives/issues/1159
				onWheel={(e) => {
					e.stopPropagation();
				}}
				onTouchMove={(e) => {
					e.stopPropagation();
				}}
			>
				<Command shouldFilter={false} loop>
					<CommandInput
						value={search}
						onValueChange={setSearch}
						placeholder={placeholder}
					/>
					<CommandList>
						{sorted.map((option) => {
							return (
								<ComboboxOption<Option>
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
						{filteredOptions.length === 0 && props.allowCreate ? (
							<ComboboxOption
								label={search}
								value={search}
								onSelect={handleNewSelect}
							/>
						) : null}
						{isLoading ? (
							<CommandLoading>Loading...</CommandLoading>
						) : null}
						{onLoadMore ? (
							<VisibilitySensor onChange={onLoadMore} />
						) : null}
					</CommandList>
				</Command>
			</PopoverContent>
		</Popover>
	);
};

interface ComboboxOptionProps<Option extends ComboboxOption> {
	isCurrent?: boolean;
	label: Option["label"];
	value: Option["value"];
	onSelect: (value: string) => void;
}

function ComboboxOption<Option extends ComboboxOption>({
	isCurrent,
	label,
	value,
	onSelect,
}: ComboboxOptionProps<Option>) {
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
