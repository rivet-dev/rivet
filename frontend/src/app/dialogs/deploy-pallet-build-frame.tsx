import type { Rivet } from "@rivet-gg/cloud";
import {
	useMutation,
	useQueryClient,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { useWatch } from "react-hook-form";
import z from "zod";
import {
	createSchemaForm,
	type DialogContentProps,
	Flex,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Frame,
	Input,
	Select,
	SelectContent,
	SelectItem,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
	toast,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { type ApiFieldMap, showApiErrorOnForm } from "@/lib/form-errors";

type WorkerPool = Rivet.v2.WorkerPoolsListResponse.Items.Item;

const NEW_POOL = "new";

const formSchema = z
	.object({
		buildId: z.string().min(1, "Select a build"),
		pool: z.string().min(1, "Select a worker pool"),
		namespace: z.string(),
		region: z.string(),
		name: z.string(),
	})
	.superRefine((values, ctx) => {
		if (values.pool !== NEW_POOL) return;
		if (!values.namespace) {
			ctx.addIssue({
				code: "custom",
				path: ["namespace"],
				message: "Select a namespace",
			});
		}
		if (!values.region) {
			ctx.addIssue({
				code: "custom",
				path: ["region"],
				message: "Select a region",
			});
		}
		if (!/^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$/.test(values.name)) {
			ctx.addIssue({
				code: "custom",
				path: ["name"],
				message:
					"Use lowercase letters, numbers, and hyphens, starting and ending with a letter or number.",
			});
		}
	});

type FormValues = z.infer<typeof formSchema>;

const API_FIELDS: ApiFieldMap<
	FormValues,
	keyof Rivet.v2.WorkerPoolsUpsertRequest | "namespace" | "region" | "pool"
> = {
	build_id: "buildId",
	namespace: "namespace",
	region: "region",
	pool: "name",
};

const { Form, Submit } = createSchemaForm(formSchema);

const poolKey = (pool: Pick<WorkerPool, "namespace" | "region" | "pool">) =>
	`${pool.namespace}/${pool.region}/${pool.pool}`;

interface DeployPalletBuildFrameContentProps extends DialogContentProps {
	cluster: string;
	buildId?: string;
}

export default function DeployPalletBuildFrameContent({
	cluster,
	buildId,
	onClose,
}: DeployPalletBuildFrameContentProps) {
	const dataProvider = useCloudDataProvider();
	const queryClient = useQueryClient();
	const { data: builds } = useSuspenseQuery(
		dataProvider.palletClusterBuildsQueryOptions({ cluster }),
	);
	const { data: pools } = useSuspenseQuery(
		dataProvider.palletClusterWorkerPoolsQueryOptions({ cluster }),
	);
	const { data: regions } = useSuspenseQuery(
		dataProvider.palletClusterRegionsQueryOptions({ cluster }),
	);
	const { data: namespaces } = useSuspenseQuery(
		dataProvider.palletClusterNamespacesQueryOptions({ cluster }),
	);

	const { mutateAsync } = useMutation({
		...dataProvider.deployPalletBuildMutationOptions(),
		meta: { hideErrorToast: true },
		onSuccess: async (data) => {
			await queryClient.invalidateQueries(
				dataProvider.palletClusterWorkerPoolsQueryOptions({ cluster }),
			);
			toast.success(`Deploying to ${data.pool}`);
			onClose?.();
		},
	});

	return (
		<Form
			revalidateMode="onChange"
			defaultValues={{
				buildId: buildId ?? "",
				pool:
					pools.length === 0
						? NEW_POOL
						: pools.length === 1
							? poolKey(pools[0])
							: "",
				namespace:
					namespaces.length === 1 ? namespaces[0].namespace : "",
				region: regions.length === 1 ? regions[0].region : "",
				name: "",
			}}
			onSubmit={async (values, form) => {
				const existing = pools.find(
					(pool) => poolKey(pool) === values.pool,
				);
				const target = existing
					? {
							namespace: existing.namespace,
							region: existing.region,
							pool: existing.pool,
						}
					: {
							namespace: values.namespace,
							region: values.region,
							pool: values.name,
						};
				if (
					!existing &&
					pools.some((pool) => poolKey(pool) === poolKey(target))
				) {
					form.setError("name", {
						message:
							"A worker pool with this name already exists in this namespace and region.",
					});
					return;
				}
				const region = regions.find((r) => r.region === target.region);
				try {
					await mutateAsync({
						cluster,
						...target,
						build_id: values.buildId,
						cidr_allowlist:
							existing?.cidr_allowlist ??
							region?.cidr_allowlist ??
							[],
						desired_count: existing?.desired_count ?? 1,
						node_template: existing?.node_template ?? {
							node_size: "small",
						},
					});
				} catch (error) {
					showApiErrorOnForm(form, error, API_FIELDS);
				}
			}}
		>
			<Frame.Header>
				<Frame.Title>Deploy build</Frame.Title>
				<Frame.Description>
					Workers in the pool are replaced with the selected build.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<FormField
						name="buildId"
						render={({ field }) => (
							<FormItem>
								<FormLabel>Build</FormLabel>
								<Select
									onValueChange={field.onChange}
									value={field.value}
								>
									<FormControl>
										<SelectTrigger>
											<SelectValue placeholder="Select a build" />
										</SelectTrigger>
									</FormControl>
									<SelectContent>
										{builds.map((build) => (
											<SelectItem
												key={build.build_id}
												value={build.build_id}
											>
												<span className="flex min-w-0 items-baseline gap-2">
													<span className="font-mono text-xs">
														{build.build_id.slice(
															0,
															8,
														)}
													</span>
													<span className="truncate text-muted-foreground">
														{build.image}
													</span>
												</span>
											</SelectItem>
										))}
									</SelectContent>
								</Select>
								<FormMessage />
							</FormItem>
						)}
					/>
					<FormField
						name="pool"
						render={({ field }) => (
							<FormItem>
								<FormLabel>Worker pool</FormLabel>
								<Select
									onValueChange={field.onChange}
									value={field.value}
								>
									<FormControl>
										<SelectTrigger>
											<SelectValue placeholder="Select a worker pool" />
										</SelectTrigger>
									</FormControl>
									<SelectContent>
										{pools.map((pool) => (
											<SelectItem
												key={poolKey(pool)}
												value={poolKey(pool)}
												disabled={pool.status.deleting}
											>
												<span className="flex items-baseline gap-2">
													{pool.pool}
													<span className="text-muted-foreground">
														{pool.namespace} ·{" "}
														{pool.region}
													</span>
												</span>
											</SelectItem>
										))}
										{pools.length ? (
											<SelectSeparator />
										) : null}
										<SelectItem value={NEW_POOL}>
											New worker pool
										</SelectItem>
									</SelectContent>
								</Select>
								<FormMessage />
							</FormItem>
						)}
					/>
					<NewPoolFields namespaces={namespaces} regions={regions} />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<Submit type="submit" allowPristine>
					Deploy
				</Submit>
			</Frame.Footer>
		</Form>
	);
}

function NewPoolFields({
	namespaces,
	regions,
}: {
	namespaces: Rivet.v2.NamespacesListResponse.Items.Item[];
	regions: Rivet.v2.RegionsListResponse.Items.Item[];
}) {
	const pool = useWatch<FormValues, "pool">({ name: "pool" });
	if (pool !== NEW_POOL) return null;

	return (
		<div className="grid gap-4 sm:grid-cols-2">
			<FormField
				name="namespace"
				render={({ field }) => (
					<FormItem>
						<FormLabel>Namespace</FormLabel>
						<Select
							onValueChange={field.onChange}
							value={field.value}
						>
							<FormControl>
								<SelectTrigger>
									<SelectValue placeholder="Select a namespace" />
								</SelectTrigger>
							</FormControl>
							<SelectContent>
								{namespaces.map((namespace) => (
									<SelectItem
										key={namespace.namespace}
										value={namespace.namespace}
									>
										{namespace.namespace}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<FormMessage />
					</FormItem>
				)}
			/>
			<FormField
				name="region"
				render={({ field }) => (
					<FormItem>
						<FormLabel>Region</FormLabel>
						<Select
							onValueChange={field.onChange}
							value={field.value}
						>
							<FormControl>
								<SelectTrigger>
									<SelectValue placeholder="Select a region" />
								</SelectTrigger>
							</FormControl>
							<SelectContent>
								{regions.map((region) => (
									<SelectItem
										key={region.region}
										value={region.region}
									>
										{region.region}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<FormMessage />
					</FormItem>
				)}
			/>
			<FormField
				name="name"
				render={({ field }) => (
					<FormItem className="sm:col-span-2">
						<FormLabel>Name</FormLabel>
						<FormControl>
							<Input
								className="font-mono"
								autoComplete="off"
								placeholder="default"
								{...field}
							/>
						</FormControl>
						<FormMessage />
					</FormItem>
				)}
			/>
		</div>
	);
}
