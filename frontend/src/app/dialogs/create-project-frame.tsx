import { Rivet } from "@rivet-gg/cloud";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import * as CreateProjectForm from "@/app/forms/create-project-form";
import { Flex, Frame } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
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
	plan,
}: {
	name?: string;
	organization?: string;
	/**
	 * Billing plan chosen before creation. Applied to the project right after
	 * it is created: directly when the org can change plans, otherwise by
	 * redirecting to the Stripe payment page with a return URL back to the
	 * new namespace.
	 */
	plan?: Rivet.BillingPlan;
	onSuccess?: (
		data: Rivet.ProjectsCreateResponse,
		vars: CreateProjectSuccessVars,
	) => void;
}) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const provider = useCloudDataProvider();

	const defaultOrg = useDefaultOrg();

	const { mutateAsync } = useMutation({
		...provider.createProjectMutationOptions(),
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

	return (
		<CreateProjectForm.Form
			onSubmit={async (values) => {
				const result = await mutateAsync({
					displayName: values.name,
					organization: values.organization,
				});

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

				// Apply the pre-selected paid plan now that the project
				// exists. When the org has no payment method yet, hand the
				// browser off to the Stripe payment page, which returns to
				// the new namespace with the plan still to apply.
				if (
					features.billing &&
					plan &&
					plan !== Rivet.BillingPlan.Free
				) {
					const target = {
						organization: values.organization,
						project: result.project.name,
					};
					// `mutationOptions` types `mutationFn` as optional even
					// though both providers always define it.
					const { mutationFn: setPlan } =
						provider.changeProjectBillingPlanMutationOptions(
							target,
						);
					const { mutationFn: createPaymentSession } =
						provider.billingCustomerPortalSessionMutationOptions({
							organization: values.organization,
						});
					if (!setPlan || !createPaymentSession) {
						throw new Error(
							"billing mutations missing mutationFn",
						);
					}
					try {
						const details = await queryClient.fetchQuery(
							provider.billingDetailsQueryOptions(target),
						);
						if (details.billing.canChangePlan) {
							await setPlan({ plan });
						} else if (namespace) {
							const returnUrl = new URL(
								`/orgs/${values.organization}/projects/${result.project.name}/ns/${namespace}`,
								window.location.origin,
							);
							returnUrl.searchParams.set("applyPlan", plan);
							const url = await createPaymentSession({
								returnUrl: returnUrl.toString(),
							});
							window.location.assign(url);
							// Stop here: the browser is leaving for the
							// payment page.
							return;
						}
					} catch (error) {
						// The project exists even if plan setup failed, so
						// continue to the namespace and let the user retry
						// from billing settings.
						console.warn("failed to apply selected plan", error);
						toast.error(
							"Could not apply the selected plan. You can upgrade from billing settings.",
						);
					}
				}

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
			}}
			defaultValues={{
				name: name,
				organization: organization ?? defaultOrg ?? "",
			}}
		>
			<Frame.Header>
				<Frame.Title>Create new project</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<CreateProjectForm.Organization />
					<CreateProjectForm.Name />
				</Flex>
			</Frame.Content>
			<Frame.Footer className="flex-row justify-end">
				<CreateProjectForm.DefaultSubmit />
			</Frame.Footer>
		</CreateProjectForm.Form>
	);
}
