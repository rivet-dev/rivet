"use client";
import { Icon, faTrash } from "@rivet-gg/icons";
import { type ChangeEvent, useState } from "react";
import { FormControl, FormItem, FormMessage } from "../../ui/form";
import { Input } from "../../ui/input";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";

export default function AutoFormFile({
	label,
	isRequired,
	fieldConfigItem,
	fieldProps,
	field,
}: AutoFormInputComponentProps) {
	const { showLabel: _showLabel, ...fieldPropsWithoutShowLabel } = fieldProps;
	const showLabel = _showLabel === undefined ? true : _showLabel;
	const [file, setFile] = useState<string | null>(null);
	const [fileName, setFileName] = useState<string | null>(null);
	const handleFileChange = (e: ChangeEvent<HTMLInputElement>) => {
		const file = e.target.files?.[0];

		if (file) {
			const reader = new FileReader();
			reader.onloadend = () => {
				setFile(reader.result as string);
				setFileName(file.name);
				field.onChange(reader.result as string);
			};
			reader.readAsDataURL(file);
		}
	};

	const handleRemoveClick = () => {
		setFile(null);
	};

	return (
		<FormItem>
			{showLabel && (
				<AutoFormLabel
					label={fieldConfigItem?.label || label}
					isRequired={isRequired}
				/>
			)}
			{!file && (
				<FormControl>
					<Input
						type="file"
						{...fieldPropsWithoutShowLabel}
						onChange={handleFileChange}
						value={""}
					/>
				</FormControl>
			)}
			{file && (
				<div className="flex h-[40px] w-full flex-row items-center justify-between space-x-2 rounded-sm border p-2 text-black focus-visible:ring-0 focus-visible:ring-offset-0 dark:bg-white dark:text-black dark:focus-visible:ring-0 dark:focus-visible:ring-offset-0">
					<p>{fileName}</p>
					<button
						onClick={handleRemoveClick}
						type="button"
						aria-label="Remove image"
					>
						<Icon icon={faTrash} />
					</button>
				</div>
			)}
			<AutoFormTooltip fieldConfigItem={fieldConfigItem} />
			<FormMessage />
		</FormItem>
	);
}
