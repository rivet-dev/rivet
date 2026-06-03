import { type Rivet } from "@rivetkit/engine-api-full";
import { infiniteQueryOptions, useInfiniteQuery } from "@tanstack/react-query";
import { useCallback, useMemo } from "react";
import { Combobox } from "@/components";
import { useEngineCompatDataProvider } from "./data-provider";

interface AllRunnerSelectProps {
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

const emptyRunnerConfigsQueryOptions = infiniteQueryOptions({
	queryKey: ["noop-runner-configs"] as readonly unknown[],
	queryFn: async (): Promise<Rivet.RunnerConfigsListResponse> => ({
		runnerConfigs: {},
		pagination: {},
	}),
	initialPageParam: undefined as string | undefined,
	getNextPageParam: () => undefined,
	select: (data) =>
		data.pages.flatMap((page) => Object.keys(page.runnerConfigs)),
});

export const useAllRunners = () => {
	const dataProvider = useEngineCompatDataProvider();
	const hasRunnerNames = "runnerNamesQueryOptions" in dataProvider;
	const {
		data: runners = [],
		hasNextPage: runnersHasNextPage,
		fetchNextPage: fetchNextRunnersPage,
		isLoading: runnersIsLoading,
		isFetchingNextPage: runnersIsFetchingNextPage,
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

	const hasRunnerConfigs = "runnerConfigsQueryOptions" in dataProvider;
	const {
		data: serverlessRunners = [],
		hasNextPage: serverlessHasNextPage,
		fetchNextPage: fetchNextServerlessPage,
		isLoading: serverlessIsLoading,
		isFetchingNextPage: serverlessIsFetchingNextPage,
	} = useInfiniteQuery<
		Rivet.RunnerConfigsListResponse,
		Error,
		string[],
		readonly unknown[],
		string | undefined
	>({
		...(hasRunnerConfigs
			? {
					...dataProvider.runnerConfigsQueryOptions({
						variant: "serverless",
					}),
					select: (data: {
						pages: { runnerConfigs: Record<string, unknown> }[];
					}) =>
						data.pages.flatMap((page) =>
							Object.keys(page.runnerConfigs),
						),
				}
			: emptyRunnerConfigsQueryOptions),
		enabled: hasRunnerConfigs,
	});

	const allRunners = useMemo(() => {
		// combine two arrays and remove duplicates
		const combined = [...runners, ...serverlessRunners];
		return Array.from(new Set(combined));
	}, [runners, serverlessRunners]);

	const fetchNextPage = useCallback(() => {
		fetchNextRunnersPage();
		fetchNextServerlessPage();
	}, [fetchNextRunnersPage, fetchNextServerlessPage]);

	return {
		hasNextPage: runnersHasNextPage || serverlessHasNextPage,
		isLoading: runnersIsLoading || serverlessIsLoading,
		isFetchingNextPage:
			runnersIsFetchingNextPage || serverlessIsFetchingNextPage,
		fetchNextPage,
		data: allRunners,
	};
};

export function AllRunnerSelect({
	onValueChange,
	value,
}: AllRunnerSelectProps) {
	const {
		data: runners,
		hasNextPage,
		isFetchingNextPage,
		isLoading,
		fetchNextPage,
	} = useAllRunners();

	const builds = useMemo(() => {
		const options = runners.map((runner) => {
			return {
				label: runner,
				value: runner,
			};
		});
		return options;
	}, [runners]);

	return (
		<Combobox
			placeholder="Choose a runner..."
			options={builds}
			value={value}
			onValueChange={onValueChange}
			className="w-full"
			isLoading={isFetchingNextPage || isLoading}
			onLoadMore={hasNextPage ? fetchNextPage : undefined}
		/>
	);
}
