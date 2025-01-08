import { FormControl, FormItem, FormMessage } from "../../ui/form";
import { Input } from "../../ui/input";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";

export default function AutoFormInput({
	label,
	isRequired,
	fieldConfigItem,
	fieldProps,
}: AutoFormInputComponentProps) {
	const { showLabel: _showLabel, ...fieldPropsWithoutShowLabel } = fieldProps;
	const showLabel = _showLabel === undefined ? true : _showLabel;
	const type = fieldProps.type || "text";

	return (
		<div className="flex flex-row  items-center space-x-2">
			<FormItem className="flex w-full flex-col justify-start">
				{showLabel && (
					<AutoFormLabel
						label={fieldConfigItem?.label || label}
						isRequired={isRequired}
					/>
				)}
				<FormControl>
					<Input type={type} {...fieldPropsWithoutShowLabel} />
				</FormControl>
				<AutoFormTooltip fieldConfigItem={fieldConfigItem} />
				<FormMessage />
			</FormItem>
		</div>
	);
}
