"use client";
import { useState } from "react";
import type * as z from "zod";
import { FormControl, FormItem, FormMessage } from "../../ui/form";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../ui/select";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";
import { getBaseSchema } from "../utils";
import AutoFormObject from "./object";

export default function AutoFormUnion({
	label,
	isRequired,
	field,
	path,
	fieldConfigItem,
	zodItem,
	fieldProps,
}: AutoFormInputComponentProps & { path?: string[] }) {
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	const def = (getBaseSchema(zodItem) as unknown as z.ZodUnion<any>)._def;

	const [selected, setSelected] = useState(() => {
		if (field.value === null) {
			return -1;
		}

		// biome-ignore lint/suspicious/noExplicitAny: FIXME
		const selected = def.options.findIndex((option: any) => {
			try {
				option.parse(field.value);
				return true;
			} catch {
				return false;
			}
		});

		return selected;
	});

	return (
		<>
			<FormItem>
				<AutoFormLabel
					label={fieldConfigItem?.label || label}
					isRequired={isRequired}
				/>
				<FormControl>
					<Select
						onValueChange={(value) => {
							const newSelected = Number.parseInt(value, 10);
							setSelected(newSelected);
							const newConfig = Object.fromEntries(
								Object.keys(def.options[newSelected].shape).map(
									(key) => [key, {}],
								),
							);
							field.onChange(newConfig);
						}}
						value={`${selected}`}
					>
						<SelectTrigger className={fieldProps.className}>
							<SelectValue
								placeholder={
									fieldConfigItem.inputProps?.placeholder
								}
							>
								{selected === -1
									? "Select an option"
									: `Option #${selected + 1}`}
							</SelectValue>
						</SelectTrigger>
						<SelectContent>
							{
								// biome-ignore lint/suspicious/noExplicitAny: FIXME
								def.options.map((_: any, index: number) => (
									<SelectItem
										value={`${index}`}
										// biome-ignore lint/suspicious/noArrayIndexKey: FIXME
										key={index}
									>
										Option #{index + 1}
									</SelectItem>
								))
							}
						</SelectContent>
					</Select>
				</FormControl>
				<AutoFormTooltip fieldConfigItem={fieldConfigItem} />
				<FormMessage />
			</FormItem>
			{selected !== null ? (
				<AutoFormObject schema={def.options[selected]} path={path} />
			) : null}
		</>
	);
}
