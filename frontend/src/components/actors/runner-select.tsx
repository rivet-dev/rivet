import { useInfiniteQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Combobox } from "@/components";
import { useEngineCompatDataProvider } from "./data-provider";

interface ConnectedRunnerSelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

export function ConnectedRunnerSelect({
	onValueChange,
	value,
}: ConnectedRunnerSelectProps) {
	const {
		data = [],
		hasNextPage,
		fetchNextPage,
		isLoading,
		isFetchingNextPage,
	} = useInfiniteQuery(
		useEngineCompatDataProvider().runnerNamesQueryOptions(),
	);

	const builds = useMemo(() => {
		const runners = data.map((runner) => {
			return {
				label: runner,
				value: runner,
			};
		});
		return runners;
	}, [data]);

	const handleValueChange = (value: string) => {
		onValueChange(value);
	};

	return (
		<Combobox
			placeholder="Choose a runner..."
			options={builds}
			value={value}
			onValueChange={handleValueChange}
			className="w-full"
			isLoading={isFetchingNextPage || isLoading}
			onLoadMore={hasNextPage ? fetchNextPage : undefined}
		/>
	);
}
