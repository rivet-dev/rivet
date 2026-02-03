import {
	Combobox,
	defaultRenderCurrentOptions,
} from "@/components/ui/combobox";

interface Namespace {
	id: string;
	name: string;
	displayName: string;
}

interface NamespaceFilterComboboxProps {
	namespaces: Namespace[];
	value: string[];
	onValueChange: (value: string[]) => void;
}

export function NamespaceFilterCombobox({
	namespaces,
	value,
	onValueChange,
}: NamespaceFilterComboboxProps) {
	const options = namespaces.map((ns) => ({
		value: ns.name,
		label: ns.displayName || ns.name,
	}));

	return (
		<Combobox
			multiple
			options={options}
			value={value}
			onValueChange={onValueChange}
			placeholder="Filter namespaces..."
			renderCurrentOptions={(currentOptions) =>
				currentOptions.length === options.length
					? "All Namespaces"
					: defaultRenderCurrentOptions(currentOptions, 1)
			}
		/>
	);
}
