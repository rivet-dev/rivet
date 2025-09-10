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
				label: build.name,
				value: build.name,
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
			filter={(option, search) => option.build.name.includes(search)}
			className="w-full"
		/>
	);
}
