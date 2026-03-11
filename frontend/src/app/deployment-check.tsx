import { faCheck, Icon } from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { useController } from "react-hook-form";
import { Uptime } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

export function DeploymentCheck({
	pollIntervalMs = 3_000,
	validate,
}: {
	pollIntervalMs?: number;
	validate: (
		data:
			| [string, Rivet.RunnerConfigsListResponseRunnerConfigsValue][]
			| undefined,
	) => boolean;
}) {
	const dataProvider = useEngineCompatDataProvider();

	const { data, isRefetching } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		retry: 0,
		refetchInterval: pollIntervalMs,
		maxPages: Infinity,
	});
	const {
		field: { onChange },
	} = useController({ name: "success" });

	const isSuccess = validate(data);

	useEffect(() => {
		onChange(isSuccess);
	}, [isSuccess, onChange]);

	if (isSuccess) {
		return (
			<>
				<Icon icon={faCheck} className="mr-1.5 text-primary" />
				Deployment successful!
			</>
		);
	}
	return (
		<>
			Waiting for deployment... (
			{isRefetching ? (
				<span>Checking...</span>
			) : (
				<span>
					Checking again in{" "}
					<Uptime
						createTs={new Date(Date.now() + pollIntervalMs + 1000)}
						showSeconds
						absolute
					/>
				</span>
			)}
			)
		</>
	);
}
