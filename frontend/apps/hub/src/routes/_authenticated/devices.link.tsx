import { Onboarding } from "@/components/onboarding/onboarding";
import * as DeviceLinkForm from "@/domains/auth/forms/device-link-form";
import {
	deviceLinkTokenQueryOptions,
	useCompleteDeviceLinkMutation,
} from "@/domains/auth/queries";
import { projectsByGroupQueryOptions } from "@/domains/project/queries";
import { queryClient } from "@/queries/global";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	Flex,
	Page,
	Text,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, createFileRoute, notFound } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";

function DeviceLinkTokenRoute() {
	const navigate = Route.useNavigate();
	const { token, newbie } = Route.useSearch();

	const { mutateAsync, isSuccess } = useCompleteDeviceLinkMutation();
	const { data: groups, refetch } = useSuspenseQuery(
		projectsByGroupQueryOptions(),
	);

	if (groups.length === 0 || newbie) {
		return (
			<Onboarding
				onFinish={async () => {
					await refetch();
					if (newbie) {
						navigate({
							to: ".",
							search: (prev) => ({ ...prev, newbie: undefined }),
						});
					}
				}}
			/>
		);
	}

	if (isSuccess) {
		return (
			<Page className="relative h-full flex flex-col items-center justify-center">
				<Card>
					<CardHeader>
						<CardTitle>Project Linked Successfully</CardTitle>
					</CardHeader>
					<CardContent>
						<Text>
							You may safely close this tab and return to your
							project.
						</Text>
					</CardContent>
					<CardFooter>
						<Button asChild variant="secondary">
							<Link to="/">Home</Link>
						</Button>
					</CardFooter>
				</Card>
			</Page>
		);
	}

	return (
		<DeviceLinkForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					deviceLinkToken: token,
					gameId: values.projectId,
				});
			}}
			defaultValues={{
				token,
				projectId:
					groups.length === 1 && groups[0].projects.length === 1
						? groups[0].projects[0].gameId
						: "",
			}}
		>
			<Page className="relative h-full flex flex-col items-center justify-center">
				<Card>
					<CardHeader>
						<CardTitle>Link project</CardTitle>
						<CardDescription>
							Link your project to your device here to continue
							with Rivet setup.
						</CardDescription>
					</CardHeader>
					<CardContent>
						<DeviceLinkForm.Project />
					</CardContent>
					<CardFooter>
						<Flex gap="4">
							<Button asChild variant="secondary">
								<Link to="/">Cancel</Link>
							</Button>
							<DeviceLinkForm.Submit>
								Continue
							</DeviceLinkForm.Submit>
						</Flex>
					</CardFooter>
				</Card>
			</Page>
		</DeviceLinkForm.Form>
	);
}

export const searchSchema = z.object({
	token: z.string(),
	newbie: z.boolean().optional(),
});

export const Route = createFileRoute("/_authenticated/devices/link")({
	validateSearch: zodValidator(searchSchema),
	component: DeviceLinkTokenRoute,
	staticData: {
		layout: "onboarding",
	},
	beforeLoad: async ({ search: { token } }) => {
		try {
			const response = await queryClient.fetchQuery(
				deviceLinkTokenQueryOptions(token),
			);
			if (!response) {
				throw notFound();
			}
		} catch {
			throw notFound();
		}
	},
});
