import { useInfiniteQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Combobox } from "@/components";
import { useManager } from "./manager-context";

interface BuildSelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

export function BuildSelect({ onValueChange, value }: BuildSelectProps) {
	const { data = [] } = useInfiniteQuery(useManager().buildsQueryOptions());

	const builds = useMemo(() => {
		return data.map((build) => {
			return {
				label: (
					<div>
						<div className="flex flex-col gap-0.5 mb-1 text-left">
							<div className="font-semibold">
								{build.tags?.name || build.name}
							</div>
							{build.createdAt ? (
								<div className="text-xs">
									Created: {build.createdAt.toLocaleString()}
								</div>
							) : null}
						</div>
					</div>
				),
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
