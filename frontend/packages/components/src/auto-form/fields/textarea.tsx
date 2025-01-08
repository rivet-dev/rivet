import { FormControl, FormItem, FormMessage } from "../../ui/form";
import { Textarea } from "../../ui/textarea";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";

export default function AutoFormTextarea({
	label,
	isRequired,
	fieldConfigItem,
	fieldProps,
}: AutoFormInputComponentProps) {
	const { showLabel: _showLabel, ...fieldPropsWithoutShowLabel } = fieldProps;
	const showLabel = _showLabel === undefined ? true : _showLabel;
	return (
		<FormItem>
			{showLabel && (
				<AutoFormLabel
					label={fieldConfigItem?.label || label}
					isRequired={isRequired}
				/>
			)}
			<FormControl>
				<Textarea {...fieldPropsWithoutShowLabel} />
			</FormControl>
			<AutoFormTooltip fieldConfigItem={fieldConfigItem} />
			<FormMessage />
		</FormItem>
	);
}
