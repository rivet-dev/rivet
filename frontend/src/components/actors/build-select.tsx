import { useInfiniteQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Combobox } from "@/components";
import { useDataProvider } from "./data-provider";

interface BuildSelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

export function BuildSelect({ onValueChange, value }: BuildSelectProps) {
	const { data = [] } = useInfiniteQuery(
		useDataProvider().buildsQueryOptions(),
	);

	const builds = useMemo(() => {
		return data.map((build) => {
			return {
				label: build.id,
				value: build.id,
				build,
			};
		});
	}, [data]);

	return (
		<Combobox
			placeholder="Choose a name..."
			options={builds}
			value={value}
			onValueChange={onValueChange}
			filter={(option, search) => option.value.includes(search)}
			className="w-full"
		/>
	);
}
