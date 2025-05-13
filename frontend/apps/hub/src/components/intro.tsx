import * as GroupCreateForm from "@/domains/group/forms/group-create-form";
import { useGroupCreateMutation } from "@/domains/group/queries";
import * as GroupCreateProjectForm from "@/domains/project/forms/group-create-project-form";
import {
	projectsByGroupQueryOptions,
	projectsQueryOptions,
	useProjectCreateMutation,
} from "@/domains/project/queries";
import { queryClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api-full";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@rivet-gg/components";
import * as Sentry from "@sentry/react";
import { useSuspenseQuery } from "@tanstack/react-query";
import { motion } from "framer-motion";
import { useState } from "react";

enum Step {
	CreateGroup = 0,
	CreateProject = 1,
}

interface IntroProps {
	initialStep?: Step;
	initialProjectName?: string;
	onFinish?: (project: Rivet.game.GameSummary) => Promise<void> | void;
}

export function Intro({
	initialStep,
	initialProjectName,
	onFinish,
}: IntroProps) {
	const { mutateAsync, data: createdGroupResponse } =
		useGroupCreateMutation();
	const { mutateAsync: createProject, data: projectCreationData } =
		useProjectCreateMutation();

	const { data } = useSuspenseQuery(projectsByGroupQueryOptions());

	const project =
		data
			.flatMap((team) => team.projects)
			.find(
				(project) => project.gameId === projectCreationData?.gameId,
			) || data.find((team) => team.projects.length > 0)?.projects[0];

	const [step, setStep] = useState<Step>(
		() => initialStep ?? (!project ? Step.CreateGroup : Step.CreateProject),
	);

	const groupId = createdGroupResponse?.groupId || project?.developer.groupId;

	if (step === Step.CreateProject) {
		return (
			<Card asChild className="max-w-xl mx-auto">
				<motion.div layoutId="card">
					<motion.div
						key="create-project"
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
						exit={{ opacity: 0 }}
					>
						<GroupCreateProjectForm.Form
							defaultValues={{
								slug: "",
								name: initialProjectName ?? "",
							}}
							onSubmit={async (values) => {
								const { gameId } = await createProject({
									displayName: values.name,
									nameId: values.slug,
									developerGroupId:
										createdGroupResponse?.groupId ||
										data[0].groupId,
								});

								const { games } = await queryClient.fetchQuery(
									projectsQueryOptions(),
								);

								const project = games.find(
									(game) => game.gameId === gameId,
								);

								if (!project) {
									Sentry.captureMessage(
										"Project not found after creation",
										"fatal",
									);
									return;
								}

								return onFinish?.(project);
							}}
						>
							<CardHeader>
								<CardTitle>Create Project</CardTitle>
								<CardDescription>
									You've created a team! Now you can create
									projects and invite teammates.
								</CardDescription>
							</CardHeader>
							<CardContent>
								<div className="grid grid-cols-[auto_auto_min-content] items-center gap-4 ">
									{initialProjectName ? (
										<GroupCreateProjectForm.SetValue
											name="name"
											value={initialProjectName}
										/>
									) : null}
									<GroupCreateProjectForm.Name className="contents space-y-0" />
									<GroupCreateProjectForm.Slug className="contents space-y-0" />
									<GroupCreateProjectForm.Submit
										type="submit"
										className="col-start-3 row-start-2"
									>
										Create
									</GroupCreateProjectForm.Submit>
								</div>
							</CardContent>
						</GroupCreateProjectForm.Form>
					</motion.div>
				</motion.div>
			</Card>
		);
	}

	return (
		<Card asChild className="max-w-md mx-auto">
			<motion.div layoutId="card">
				<motion.div
					key="create-group"
					initial={{ opacity: 0 }}
					animate={{ opacity: 1 }}
					exit={{ opacity: 0 }}
				>
					<GroupCreateForm.Form
						onSubmit={async (values) => {
							await mutateAsync({
								displayName: values.name,
							});
							setStep(Step.CreateProject);
						}}
						defaultValues={{ name: "" }}
					>
						<CardHeader>
							<CardTitle>Create Team</CardTitle>
							<CardDescription>
								Before you start, you need to create a team.
								This will allow you to create projects and
								invite teammates.
							</CardDescription>
						</CardHeader>
						<CardContent>
							<div className="grid grid-cols-[auto_min-content] items-center gap-4 ">
								<GroupCreateForm.Name className="contents space-y-0" />
								<GroupCreateForm.Submit
									type="submit"
									className="col-start-2 row-start-2"
								>
									Create
								</GroupCreateForm.Submit>
							</div>
						</CardContent>
					</GroupCreateForm.Form>
				</motion.div>
			</motion.div>
		</Card>
	);
}
