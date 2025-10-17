import { useInfiniteQuery } from "@tanstack/react-query";
import { useCallback, useMemo, useState } from "react";
import { Combobox } from "@/components";
import { useEngineCompatDataProvider } from "./data-provider";

interface AllRunnerSelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

export const useAllRunners = () => {
	const {
		data: runners = [],
		hasNextPage: runnersHasNextPage,
		fetchNextPage: fetchNextRunnersPage,
		isLoading: runnersIsLoading,
		isFetchingNextPage: runnersIsFetchingNextPage,
	} = useInfiniteQuery(
		useEngineCompatDataProvider().runnerNamesQueryOptions(),
	);

	const {
		data: serverlessRunners = [],
		hasNextPage: serverlessHasNextPage,
		fetchNextPage: fetchNextServerlessPage,
		isLoading: serverlessIsLoading,
		isFetchingNextPage: serverlessIsFetchingNextPage,
	} = useInfiniteQuery({
		...useEngineCompatDataProvider().runnerConfigsQueryOptions({
			variant: "serverless",
		}),
		select: (data) =>
			data.pages.flatMap((page) => Object.keys(page.runnerConfigs)),
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
