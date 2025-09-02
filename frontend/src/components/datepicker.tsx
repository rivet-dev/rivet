"use client";

import { faCalendar, Icon } from "@rivet-gg/icons";
import { format } from "date-fns";
import type { DateRange } from "react-day-picker";
import { cn } from "./lib/utils";
import { Button } from "./ui/button";
import { Calendar } from "./ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";

export type { DateRange };

interface DatePickerProps {
	date: Date | undefined;
	onDateChange: (day: Date | undefined) => void;
	className?: string;
	presets?: { label: string; date: Date }[];
}

export function DatePicker({
	className,
	date,
	onDateChange,
	presets,
}: DatePickerProps) {
	return (
		<Popover>
			<PopoverTrigger asChild>
				<Button
					variant={"outline"}
					className={cn(
						className,
						"w-[240px] justify-start text-left font-normal",
						!date && "text-muted-foreground",
					)}
				>
					<Icon icon={faCalendar} className="mr-2 h-4 w-4" />
					{date ? format(date, "PPP") : <span>Pick a date</span>}
				</Button>
			</PopoverTrigger>
			<PopoverContent
				className="flex flex-col space-y-2 p-2 w-auto"
				align="start"
			>
				{presets ? (
					<Select
						onValueChange={(value) =>
							onDateChange(presets[Number.parseInt(value)].date)
						}
					>
						<SelectTrigger>
							<SelectValue placeholder="Select" />
						</SelectTrigger>
						<SelectContent position="popper">
							{presets.map(({ label }, index) => (
								<SelectItem key={label} value={`${index}`}>
									{label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				) : null}
				<Calendar
					mode="single"
					selected={date}
					onSelect={onDateChange}
				/>
			</PopoverContent>
		</Popover>
	);
}

interface RangeDatePickerProps {
	date: DateRange | undefined;
	onDateChange: (range: DateRange | undefined) => void;
	className?: string;
	presets?: { label: string; date: DateRange }[];
}

export function RangeDatePicker({
	date,
	className,
	onDateChange,
	presets,
}: RangeDatePickerProps) {
	return (
		<div className={cn("grid gap-2", className)}>
			<Popover>
				<PopoverTrigger asChild>
					<Button
						id="date"
						variant={"outline"}
						className={cn(
							"w-[300px] justify-start text-left font-normal",
							!date && "text-muted-foreground",
						)}
					>
						<Icon icon={faCalendar} className="mr-2 h-4 w-4" />
						{date?.from ? (
							date.to ? (
								<>
									{format(date.from, "LLL dd, y")} -{" "}
									{format(date.to, "LLL dd, y")}
								</>
							) : (
								format(date.from, "LLL dd, y")
							)
						) : (
							<span>Pick a date</span>
						)}
					</Button>
				</PopoverTrigger>
				<PopoverContent
					className="flex flex-col space-y-2 p-2 w-auto"
					align="start"
				>
					{presets ? (
						<Select
							onValueChange={(value) =>
								onDateChange(
									presets[Number.parseInt(value)].date,
								)
							}
						>
							<SelectTrigger>
								<SelectValue placeholder="Select" />
							</SelectTrigger>
							<SelectContent position="popper">
								{presets.map(({ label }, index) => (
									<SelectItem key={label} value={`${index}`}>
										{label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					) : null}
					<Calendar
						mode="range"
						defaultMonth={date?.from}
						selected={date}
						onSelect={onDateChange}
						numberOfMonths={2}
					/>
				</PopoverContent>
			</Popover>
		</div>
	);
}
