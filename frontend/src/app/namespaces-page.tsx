import { faExternalLink, faPlus, faRefresh, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Link, type LinkComponentProps } from "@tanstack/react-router";
import { match } from "ts-pattern";
import {
	Button,
	DiscreteCopyButton,
	H1,
	Skeleton,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
	Text,
	WithTooltip,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

export function NamespacesPage({ from }: { from: LinkComponentProps["from"] }) {
	const {
		data: namespaces,
		isRefetching,
		hasNextPage,
		fetchNextPage,
		isLoading,
		refetch,
	} = useInfiniteQuery(
		useEngineCompatDataProvider().namespacesQueryOptions(),
	);

	return (
		<div className="bg-card h-full border my-2 mr-2 rounded-lg">
			<div className="max-w-5xl mx-auto my-2 flex justify-between items-center px-6 py-4">
				<H1>Namespaces</H1>
				<div className="flex items-center gap-2">
					<WithTooltip
						content="Create Namespace"
						trigger={
							<Button size="icon" asChild variant="outline">
								<Link
									to="."
									search={(old) => ({
										...old,
										modal: "create-ns",
									})}
								>
									<Icon icon={faPlus} />
								</Link>
							</Button>
						}
					/>
					<WithTooltip
						content="Refresh"
						trigger={
							<Button
								size="icon"
								isLoading={isRefetching}
								variant="outline"
								onClick={() => refetch()}
							>
								<Icon icon={faRefresh} />
							</Button>
						}
					/>
				</div>
			</div>

			<hr className="mb-4" />

			<div className="p-4">
				<div className="max-w-5xl mx-auto p-2">
					<div className="border rounded-md">
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>ID</TableHead>
									<TableHead>Name</TableHead>
									<TableHead>Created</TableHead>
									<TableHead />
								</TableRow>
							</TableHeader>
							<TableBody>
								{!isLoading && namespaces?.length === 0 ? (
									<TableRow>
										<TableCell colSpan={6}>
											<Text className="text-center">
												There's no namespaces matching
												criteria.
											</Text>
										</TableCell>
									</TableRow>
								) : null}
								{isLoading ? (
									<>
										<RowSkeleton />
										<RowSkeleton />
										<RowSkeleton />
										<RowSkeleton />
										<RowSkeleton />
										<RowSkeleton />
										<RowSkeleton />
										<RowSkeleton />
									</>
								) : null}
								{namespaces?.map((namespace) => (
									<TableRow key={namespace.name}>
										<TableCell>
											<DiscreteCopyButton
												value={namespace.name}
											>
												{namespace.name}
											</DiscreteCopyButton>
										</TableCell>
										<TableCell>
											<DiscreteCopyButton
												value={namespace.displayName}
											>
												{namespace.displayName}
											</DiscreteCopyButton>
										</TableCell>
										<TableCell>
											{new Date(
												namespace.createdAt,
											).toLocaleString()}
										</TableCell>
										<TableCell>
											<Button variant="ghost" asChild>
												<Link
													to={
														__APP_TYPE__ === "cloud"
															? "/orgs/$organization/projects/$project/ns/$namespace"
															: "/ns/$namespace"
													}
													from={from}
													params={{
														namespace:
															namespace.name,
													}}
												>
													<Icon
														icon={faExternalLink}
													/>
												</Link>
											</Button>
										</TableCell>
									</TableRow>
								))}

								{!isLoading && hasNextPage ? (
									<TableRow>
										<TableCell colSpan={8}>
											<Button
												variant="outline"
												isLoading={isLoading}
												onClick={() => fetchNextPage()}
												disabled={!hasNextPage}
											>
												Load more
											</Button>
										</TableCell>
									</TableRow>
								) : null}
							</TableBody>
						</Table>
					</div>
				</div>
			</div>
		</div>
	);
}

function RowSkeleton() {
	return (
		<TableRow>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
			<TableCell>
				<Skeleton className="w-full h-4" />
			</TableCell>
		</TableRow>
	);
}
