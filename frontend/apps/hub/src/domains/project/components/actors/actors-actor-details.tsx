import {
	Button,
	ClickToCopy,
	Flex,
	LogsView,
	Ping,
	Skeleton,
	SmallText,
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
	Text,
} from "@rivet-gg/components";
import { ErrorBoundary } from "@sentry/react";
import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { formatISO } from "date-fns";
import { Suspense } from "react";
import {
	actorErrorsQueryOptions,
	actorQueryOptions,
	useDestroyActorMutation,
} from "../../queries";
import { ActorLogsTab } from "./actor-logs-tab";
import { ActorNetworkTab } from "./actor-network-tab";
import { ActorRegion } from "./actor-region";
import { ActorRuntimeTab } from "./actor-runtime-tab";
import { ActorStatus } from "./actor-status";
import { ActorTags } from "./actor-tags";

interface ActorsActorDetailsProps {
	projectNameId: string;
	environmentNameId: string;
	actorId: string;
}

export function ActorsActorDetails({
	projectNameId,
	environmentNameId,
	actorId,
}: ActorsActorDetailsProps) {
	const { data } = useSuspenseQuery(
		actorQueryOptions({ projectNameId, environmentNameId, actorId }),
	);

	const { data: hasError } = useQuery(
		actorErrorsQueryOptions({ projectNameId, environmentNameId, actorId }),
	);

	const currentTab = useSearch({
		from: "/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/actors",
		select: (state) => state.tab,
	});
	const navigate = useNavigate();

	const { mutate, isPending: isDestroying } = useDestroyActorMutation();

	if (!data) {
		return (
			<Flex items="center" justify="center" className="h-full">
				<Text my="10" textAlign="center">
					Actor not found.
				</Text>
			</Flex>
		);
	}

	return (
		<ErrorBoundary
			fallback={
				<Flex items="center" justify="center" className="h-full">
					<Text textAlign="center">
						An error occurred while fetching actor data.
					</Text>
				</Flex>
			}
		>
			<Flex direction="col" className="h-full w-full" pt="4">
				<Flex items="start" gap="4" px="6" className="flex-col">
					<Flex gap="2" justify="between" w="full">
						<ActorStatus className="text-sm" {...data} />
						{!data.destroyTs ? (
							<Button
								isLoading={isDestroying}
								variant="destructive"
								size="sm"
								onClick={() =>
									mutate({
										projectNameId,
										environmentNameId,
										actorId,
									})
								}
							>
								Stop
							</Button>
						) : null}
					</Flex>

					<div className="w-full">
						<Flex
							direction="col"
							gap="2"
							className="flex-1 min-w-0"
							w="full"
						>
							<ActorTags className="justify-start" {...data} />
						</Flex>
					</div>
					<div className="flex  w-full flex-wrap text-sm gap-4 border px-3 py-2 rounded-md -mx-3 box-content">
						<div className="shrink-0 flex gap-2 items-center justify-center">
							<p className="">Region </p>
							<SmallText className="text-xs text-muted-foreground">
								<ActorRegion
									showLabel="abbreviated"
									projectNameId={projectNameId}
									environmentNameId={environmentNameId}
									regionId={data.region}
								/>
							</SmallText>
						</div>
						<ClickToCopy value={data.id}>
							<button
								type="button"
								className="shrink-0 flex gap-2 items-center justify-center"
							>
								<p className="">ID </p>
								<SmallText className="font-mono  text-muted-foreground text-xs">
									{data.id.split("-")[0]}
								</SmallText>
							</button>
						</ClickToCopy>
						<ClickToCopy value={formatISO(data.createdAt)}>
							<button
								type="button"
								className="shrink-0 flex gap-2 items-center justify-center"
							>
								<p className=" ">Created </p>
								<SmallText className=" text-xs  text-muted-foreground">
									{formatISO(data.createdAt)}
								</SmallText>
							</button>
						</ClickToCopy>

						<ClickToCopy
							value={
								data.destroyTs ? formatISO(data.destroyTs) : "-"
							}
						>
							<button
								type="button"
								className="shrink-0 flex gap-2 items-center"
							>
								<p className="">Destroyed </p>
								<SmallText className="text-xs  text-muted-foreground ">
									{data.destroyTs
										? formatISO(data.destroyTs)
										: "-"}
								</SmallText>
							</button>
						</ClickToCopy>
					</div>
				</Flex>

				<Tabs
					value={currentTab}
					onValueChange={(tab) => {
						navigate({
							to: ".",
							search: (old) => ({
								...old,
								tab,
							}),
						});
					}}
					defaultValue="output"
					className="flex-1 min-h-0 flex flex-col mt-4"
				>
					<TabsList className="overflow-auto">
						<TabsTrigger value="output">Output</TabsTrigger>
						<TabsTrigger value="error">
							<span className="relative">
								Error
								{hasError ? (
									<Ping variant="destructive" />
								) : null}
							</span>
						</TabsTrigger>
						<TabsTrigger value="runtime">Runtime</TabsTrigger>
						<TabsTrigger value="network">Network</TabsTrigger>
					</TabsList>
					<TabsContent
						value="output"
						className="min-h-0 flex-1 mt-0 p-4"
					>
						<ErrorBoundary
							fallback={
								<Flex
									items="center"
									justify="center"
									className="h-full"
								>
									<Text textAlign="center">
										An error occurred while fetching
										actors's logs.
									</Text>
								</Flex>
							}
						>
							<Suspense fallback={<ActorLogsTab.Skeleton />}>
								<ActorLogsTab
									createdAt={data.createdAt}
									projectNameId={projectNameId}
									environmentNameId={environmentNameId}
									actorId={actorId}
									logType="std_out"
								/>
							</Suspense>
						</ErrorBoundary>
					</TabsContent>
					<TabsContent
						value="error"
						className="min-h-0 flex-1 mt-0 p-4"
					>
						<ErrorBoundary
							fallback={
								<Flex
									items="center"
									justify="center"
									className="h-full"
								>
									<Text textAlign="center">
										An error occurred while fetching actor's
										logs.
									</Text>
								</Flex>
							}
						>
							<Suspense fallback={<ActorLogsTab.Skeleton />}>
								<ActorLogsTab
									createdAt={data.createdAt}
									projectNameId={projectNameId}
									environmentNameId={environmentNameId}
									actorId={actorId}
									logType="std_err"
								/>
							</Suspense>
						</ErrorBoundary>
					</TabsContent>
					<TabsContent
						value="runtime"
						className="min-h-0 flex-1 mt-0"
					>
						<ActorRuntimeTab
							projectNameId={projectNameId}
							environmentNameId={environmentNameId}
							{...data}
						/>
					</TabsContent>
					<TabsContent
						value="network"
						className="min-h-0 flex-1 mt-0"
					>
						<ActorNetworkTab {...data} />
					</TabsContent>
				</Tabs>
			</Flex>
		</ErrorBoundary>
	);
}

ActorsActorDetails.Skeleton = () => {
	return (
		<Flex className="h-full flex-col">
			<div className="flex flex-col gap-4 px-4 flex-wrap">
				<Skeleton className="mt-3 mx-auto h-10 w-full" />
				<div className="flex gap-2 items-center">
					<Skeleton className="h-6 w-1/4" />
					<Skeleton className="h-6 w-1/4" />
					<Skeleton className="h-6 w-1/4" />
					<Skeleton className="h-6 w-1/5" />
				</div>
				<div className="flex gap-2 flex-col">
					<Skeleton className="h-6 w-1/6" />
					<Skeleton className="h-6 w-1/4" />
				</div>
			</div>
			<div className="mt-4 flex gap-1 px-4">
				<Skeleton className="h-6 w-1/5" />
				<Skeleton className="h-6 w-1/5" />
				<Skeleton className="h-6 w-1/5" />
			</div>
			<div className="px-4 pt-4">
				<LogsView.Skeleton />
			</div>
		</Flex>
	);
};
