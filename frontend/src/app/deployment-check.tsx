import type { Rivet as RivetCloud } from "@rivet-gg/cloud";
import {
	faCheck,
	faSpinnerThird,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { useController } from "react-hook-form";
import { Ping } from "@/components";
import {
	ErrorDetails,
	useCloudNamespaceDataProvider,
} from "@/components/actors";

export function DeploymentCheck({
	pollIntervalMs = 3_000,
	validateConfig,
	validatePool,
}: {
	pollIntervalMs?: number;
	validateConfig: (
		data:
			| [string, Rivet.RunnerConfigsListResponseRunnerConfigsValue][]
			| undefined,
	) => boolean;
	validatePool?: (
		data: RivetCloud.ManagedPoolsGetResponse.ManagedPool | null | undefined,
	) => boolean;
}) {
	const dataProvider = useCloudNamespaceDataProvider();

	const { data } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		retry: 0,
		refetchInterval: pollIntervalMs,
		maxPages: Infinity,
	});

	const { data: poolData } = useQuery({
		...dataProvider.currentNamespaceManagedPoolQueryOptions({
			pool: "default",
			safe: true,
		}),
		retry: 0,
		refetchInterval: pollIntervalMs,
	});

	const {
		field: { onChange },
	} = useController({ name: "success" });

	const hasRunnerConfig = validateConfig?.(data) === true;
	const hasValidPool = validatePool?.(poolData) === true;
	const isSuccess =
		hasRunnerConfig && hasValidPool && poolData?.status === "ready";

	useEffect(() => {
		onChange(isSuccess);
	}, [isSuccess, onChange]);

	if (isSuccess) {
		return (
			<span>
				<Icon icon={faCheck} className="mr-1.5 text-primary" />
				Deployment successful!
			</span>
		);
	}

	return (
		<PoolStatus
			status={
				hasRunnerConfig && hasValidPool ? poolData?.status : undefined
			}
			error={poolData?.error?.message}
		/>
	);
}

function PoolStatus({
	status,
	error,
}: {
	status?:
		| "ready"
		| "provisioning"
		| "error"
		| "initializing"
		| "allocating"
		| "deploying"
		| "binding";
	error?: string;
}) {
	if (status === "ready") {
		return (
			<span>
				<Icon icon={faCheck} className="mr-1.5 text-green-500" />
				Ready
			</span>
		);
	}
	if (status === "provisioning") {
		return (
			<span>
				<Icon icon={faSpinnerThird} className="mr-1.5 animate-spin" />
				Provisioning...
			</span>
		);
	}
	if (status === "error") {
		return (
			<>
				<p>
					<Icon
						icon={faTriangleExclamation}
						className="mr-1.5 text-red-500"
					/>
					Error
				</p>
				<ErrorDetails error={error} />
			</>
		);
	}
	if (status === "initializing") {
		return (
			<span>
				<Icon icon={faSpinnerThird} className="mr-1.5 animate-spin" />
				Initializing...
			</span>
		);
	}
	if (status === "allocating") {
		return (
			<span>
				<Icon icon={faSpinnerThird} className="mr-1.5 animate-spin" />
				Allocating...
			</span>
		);
	}
	if (status === "deploying") {
		return (
			<span>
				<Icon icon={faSpinnerThird} className="mr-1.5 animate-spin" />
				Deploying...
			</span>
		);
	}
	if (status === "binding") {
		return (
			<span>
				<Icon icon={faSpinnerThird} className="mr-1.5 animate-spin" />
				Binding...
			</span>
		);
	}
	return (
		<>
			<div className="relative mr-4">
				<Ping variant="pending" className="relative" />
			</div>
			<p>Waiting for deployment...</p>
		</>
	);
}
