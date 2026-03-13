import {
	faCheck,
	faRivet,
	faSpinnerThird,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { Suspense, useEffect } from "react";
import { type DialogContentProps, Frame } from "@/components";
import { Button } from "@/components/ui/button";
import {
	ErrorDetails,
	useCloudNamespaceDataProvider,
} from "@/components/actors";
import { deriveProviderFromMetadata } from "@/lib/data";
import { successfulBackendSetupEffect } from "@/lib/effects";
import { queryClient } from "@/queries/global";
import { CodeFrame, CodeGroup, CodePreview } from "@/components";
import { useRivetDsn } from "@/app/env-variables";

interface ConnectRivetFrameContentProps extends DialogContentProps {}

export default function ConnectRivetFrameContent({
	onClose,
}: ConnectRivetFrameContentProps) {
	const dataProvider = useCloudNamespaceDataProvider();

	const { mutate: upsertManagedPool } = useMutation({
		...dataProvider.upsertCurrentNamespaceManagedPoolMutationOptions(),
		onSuccess: async () => {
			await queryClient.invalidateQueries(
				dataProvider.runnerConfigsQueryOptions(),
			);
		},
	});

	useEffect(() => {
		upsertManagedPool({
			displayName: "default",
			pool: "default",
			minCount: 0,
			maxCount: 1000,
		});
	}, [upsertManagedPool]);

	const { data: runnerConfigs } = useInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
		retry: 0,
		refetchInterval: 3_000,
		maxPages: Infinity,
	});

	const { data: poolData } = useQuery({
		...dataProvider.currentNamespaceManagedPoolQueryOptions({
			pool: "default",
			safe: true,
		}),
		retry: 0,
		refetchInterval: 3_000,
	});

	const hasRunnerConfig = !!runnerConfigs?.find(([, value]) =>
		Object.values(value.datacenters).some(
			(dc) =>
				dc.serverless &&
				deriveProviderFromMetadata(dc.metadata) === "rivet",
		),
	);
	const hasValidPool = !!poolData?.config.image;
	const isSuccess =
		hasRunnerConfig && hasValidPool && poolData?.status === "ready";

	useEffect(() => {
		if (isSuccess) {
			successfulBackendSetupEffect();
		}
	}, [isSuccess]);

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<Icon icon={faRivet} />
					Connect Rivet Cloud
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<div className="flex flex-col gap-6">
					<div className="flex gap-3">
						<StepNumber n={1} />
						<div className="flex-1 min-w-0">
							<p className="font-medium mb-2">
								Copy this prompt into your coding agent
							</p>
							<CodeGroup className="my-0">
								{[
									<Suspense
										key="agent-instructions"
										fallback={
											<div className="h-32 animate-pulse bg-muted rounded" />
										}
									>
										<AgentInstructions />
									</Suspense>,
								]}
							</CodeGroup>
						</div>
					</div>
					<div className="flex gap-3">
						<StepNumber n={2} />
						<div className="flex-1 min-w-0">
							<p className="font-medium mb-2">Deploy</p>
							<div className="border rounded-md p-4">
								<PoolStatus
									status={
										hasRunnerConfig && hasValidPool
											? poolData?.status
											: undefined
									}
									error={poolData?.error?.message}
									isSuccess={isSuccess}
								/>
							</div>
						</div>
					</div>
				</div>
				{isSuccess ? (
					<Button className="w-full mt-6" onClick={onClose}>
						Done
					</Button>
				) : null}
			</Frame.Content>
		</>
	);
}

function StepNumber({ n }: { n: number }) {
	return (
		<div className="flex-shrink-0 w-6 h-6 rounded-full bg-primary/10 text-primary text-xs font-medium flex items-center justify-center mt-0.5">
			{n}
		</div>
	);
}

function AgentInstructions() {
	const dataProvider = useCloudNamespaceDataProvider();
	const { data: cloudToken } = useSuspenseQuery(
		dataProvider.createApiTokenQueryOptions({ name: "Onboarding" }),
	);

	const publishableToken = useRivetDsn({ kind: "publishable" });
	const secretToken = useRivetDsn({ kind: "secret" });

	const code = `Load the Rivet skill and then:
1. Integrate Rivet into the project
2. Set up a GitHub Actions workflow using
   the repository secret RIVET_TOKEN
   for automated deployment
3. Deploy to Rivet and configure
   the following environment variables:

  RIVET_PUBLIC_ENDPOINT=${publishableToken}
  RIVET_ENDPOINT=${secretToken}
  RIVET_TOKEN=${cloudToken}

4. Verify the deployment works end-to-end;
   fix any issues and repeat until it succeeds`;

	return (
		<CodeFrame language="markdown" code={() => code} className="m-0">
			<CodePreview
				language="markdown"
				className="text-left"
				code={code}
			/>
		</CodeFrame>
	);
}

function PoolStatus({
	status,
	error,
	isSuccess,
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
	isSuccess: boolean;
}) {
	if (isSuccess || status === "ready") {
		return (
			<span>
				<Icon icon={faCheck} className="mr-1.5 text-primary" />
				Deployment successful!
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
		<span>
			<Icon icon={faSpinnerThird} className="mr-1.5 animate-spin" />
			Waiting for deployment...
		</span>
	);
}
