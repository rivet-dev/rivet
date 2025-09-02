import { useInfiniteQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Combobox } from "@/components";
import {
	type NamespaceNameId,
	runnerNamesQueryOptions,
} from "@/queries/manager-engine";

interface RunnerSelectProps {
	onValueChange: (value: string) => void;
	value: string;
	namespace: NamespaceNameId;
}

export function RunnerSelect({
	onValueChange,
	value,
	namespace,
}: RunnerSelectProps) {
	const {
		data = [],
		hasNextPage,
		fetchNextPage,
		isLoading,
		isFetchingNextPage,
	} = useInfiniteQuery(runnerNamesQueryOptions({ namespace }));

	const [newRunner, setNewRunner] = useState<string | null>(null);

	const builds = useMemo(() => {
		const runners = data.map((runner) => {
			return {
				label: runner,
				value: runner,
			};
		});

		if (newRunner) {
			runners.push({
				label: newRunner,
				value: newRunner,
			});
		}

		return runners;
	}, [data, newRunner]);

	const handleNewSelect = (value: string) => {
		setNewRunner(value);
	};

	const handleValueChange = (value: string) => {
		if (value !== newRunner) {
			setNewRunner(null);
		}
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
			allowCreate
			onCreateOption={handleNewSelect}
		/>
	);
}
