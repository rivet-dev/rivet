import type { Rivet } from "@rivet-gg/cloud";
import { faArrowUpRightFromSquare, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useContext, useRef } from "react";
import * as CreateProjectForm from "@/app/forms/create-project-form";
import { StepperForm } from "@/app/forms/stepper-form";
import { Button, Flex, Frame, toast } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { IsInModalContext } from "@/components/hooks/isomorphic-frame";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

const useDefaultOrg = () => {
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		const org = authClient.useActiveOrganization();
		return org.data?.id;
	}
	return undefined;
};

export type CreateProjectSuccessVars = {
	displayName: string;
	organization: string;
	/**
	 * Name of the namespace to land on after creation. The backend auto-creates
	 * a "Production" namespace on project create, and landing on it triggers the
	 * onboarding flow. Undefined if no namespace could be resolved.
	 */
	namespace: string | undefined;
};

export default function CreateProjectFrameContent({
	organization,
	onSuccess,
	name,
}: {
	name?: string;
	organization?: string;
	onSuccess?: (
		data: Rivet.ProjectsCreateResponse,
		vars: CreateProjectSuccessVars,
	) => void;
}) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const provider = useCloudDataProvider();
	const isInModal = useContext(IsInModalContext);

	const defaultOrg = useDefaultOrg();
	const createdProject = useRef<{
		key: string;
		project: Rivet.ProjectsCreateResponse;
	} | null>(null);

	const { mutateAsync: createProject } = useMutation({
		...provider.createProjectMutationOptions(),
	});
	const { mutateAsync: createCluster } = useMutation({
		...provider.createClusterMutationOptions(),
	});
	const { mutateAsync: setPlan } = useMutation({
		...provider.setProjectBillingPlanMutationOptions(),
	});

	// Resolve the auto-created "Production" namespace so we can land on it and
	// trigger the onboarding flow right after the project is created.
	const resolveOnboardingNamespace = async (
		organization: string,
		project: string,
	) => {
		const data = await queryClient.fetchInfiniteQuery(
			provider.orgProjectNamespacesQueryOptions({
				organization,
				project,
			}),
		);
		const namespaces = data.pages.flatMap((page) => page.namespaces);
		const production = namespaces.find(
			(ns) => ns.displayName === "Production",
		);
		return (production ?? namespaces[0])?.name;
	};

	const ensureProject = async (values: CreateProjectForm.FormValues) => {
		const key = `${values.organization}/${values.name}`;
		if (createdProject.current?.key !== key) {
			createdProject.current = {
				key,
				project: await createProject({
					displayName: values.name,
					organization: values.organization,
				}),
			};
		}
		return createdProject.current.project;
	};

	const finishProject = async (values: CreateProjectForm.FormValues) => {
		const result = await ensureProject(values);

		await queryClient.refetchQueries(
			provider.currentOrgProjectsQueryOptions(),
		);

		const namespace = await resolveOnboardingNamespace(
			values.organization,
			result.project.name,
		);

		const successVars: CreateProjectSuccessVars = {
			displayName: values.name,
			organization: values.organization,
			namespace,
		};

		if (onSuccess) {
			onSuccess(result, successVars);
			return;
		}

		if (namespace) {
			await navigate({
				to: "/orgs/$organization/projects/$project/ns/$namespace",
				params: {
					organization: values.organization,
					project: result.project.name,
					namespace,
				},
			});
			return;
		}

		await navigate({
			to: "/orgs/$organization/projects/$project",
			params: {
				organization: values.organization,
				project: result.project.name,
			},
		});
	};

	const finishCluster = async (values: CreateProjectForm.FormValues) => {
		const result = await createCluster({
			name: values.name,
			organization: values.organization,
		});

		await queryClient.refetchQueries(
			provider.currentOrgClustersQueryOptions(),
		);

		await navigate({
			to: "/orgs/$organization/clusters/$cluster",
			params: {
				organization: values.organization,
				cluster: result.cluster.name,
			},
		});
	};

	return (
		<>
			<Frame.Header className="sr-only">
				<Frame.Title>Create new project</Frame.Title>
			</Frame.Header>
			{/* The visible title comes from the stepper. The sr-only header
			    leaves no top padding in a card, and the dialog's own padding
			    puts the title level with the close button. */}
			<Frame.Content className={isInModal ? "pt-2" : "pt-6"}>
				<StepperForm
					{...CreateProjectForm.stepper}
					singlePage
					defaultValues={{
						plan: "free",
						name: name ?? "",
						organization: organization ?? defaultOrg ?? "",
					}}
					content={{
						plan: () => <CreateProjectForm.Plan />,
						details: () => (
							<Flex gap="4" direction="col">
								<CreateProjectForm.Organization />
								<CreateProjectForm.Name />
							</Flex>
						),
						payment: () => <PaymentStep />,
					}}
					onPartialSubmit={async ({ values, stepper }) => {
						if (stepper.current.id !== "details") {
							return;
						}

						if (features.byoc && values.plan === "byoc") {
							return;
						}

						const result = await ensureProject(values);

						try {
							await setPlan({
								organization: values.organization,
								project: result.project.name,
								plan: values.plan as Rivet.BillingPlan,
							});
						} catch {
							toast.error(
								"Couldn't apply the plan. Set it from the project's billing page.",
							);
						}
					}}
					onSubmit={async ({ values }) => {
						if (features.byoc && values.plan === "byoc") {
							await finishCluster(values);
							return;
						}

						await finishProject(values);
					}}
				/>
			</Frame.Content>
		</>
	);
}

function PaymentStep() {
	const provider = useCloudDataProvider();
	const { data, refetch } = useQuery(
		provider.billingCustomerPortalSessionQueryOptions(),
	);

	return (
		<Button
			type="button"
			variant="secondary"
			endIcon={<Icon icon={faArrowUpRightFromSquare} />}
			onMouseEnter={() => {
				refetch();
			}}
			onClick={() => {
				if (data) {
					window.open(data, "_blank");
				}
			}}
		>
			Add payment method
		</Button>
	);
}
