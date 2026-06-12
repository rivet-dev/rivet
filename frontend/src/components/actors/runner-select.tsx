import { type Rivet } from "@rivetkit/engine-api-full";
import { infiniteQueryOptions, useInfiniteQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { Combobox } from "@/components";
import { useEngineCompatDataProvider } from "./data-provider";

interface ConnectedRunnerSelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

const emptyRunnerNamesQueryOptions = infiniteQueryOptions({
	queryKey: ["noop-runner-names"] as readonly unknown[],
	queryFn: async (): Promise<Rivet.RunnersListNamesResponse> => ({
		names: [],
		pagination: {},
	}),
	initialPageParam: undefined as string | undefined,
	getNextPageParam: () => undefined,
	select: (data) => data.pages.flatMap((page) => page.names),
});

export function ConnectedRunnerSelect({
	onValueChange,
	value,
}: ConnectedRunnerSelectProps) {
	const dataProvider = useEngineCompatDataProvider();
	const hasRunnerNames = "runnerNamesQueryOptions" in dataProvider;
	const {
		data = [],
		hasNextPage,
		fetchNextPage,
		isLoading,
		isFetchingNextPage,
	} = useInfiniteQuery<
		Rivet.RunnersListNamesResponse,
		Error,
		string[],
		readonly unknown[],
		string | undefined
	>({
		...(hasRunnerNames
			? dataProvider.runnerNamesQueryOptions()
			: emptyRunnerNamesQueryOptions),
		enabled: hasRunnerNames,
	});

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
