import type { SubmitHandler } from "@/domains/project/forms/cdn-manage-auth-users-form";
import { queryClient } from "@/queries/global";
import type { Rivet } from "@rivet-gg/api-full";
import bcrypt from "bcryptjs";
import { useCallback } from "react";
import {
	projectEnvironmentQueryOptions,
	projectQueryOptions,
	useEnvironmentRemoveCdnAuthUserMutation,
	useEnvironmentUpdateCdnAuthUserMutation,
} from "../queries";

const SALT = bcrypt.genSaltSync(10);

type CdnEnvironmentAuthPasswordUser = Pick<
	Rivet.cloud.CdnNamespaceAuthUser,
	"user"
> & {
	password: string;
};

function computeUsersDiff(
	existingUsers: Rivet.cloud.CdnNamespaceAuthUser["user"][],
	newUsers: CdnEnvironmentAuthPasswordUser[],
) {
	const update: CdnEnvironmentAuthPasswordUser[] = [];
	const create: CdnEnvironmentAuthPasswordUser[] = [];
	const errors: { idx: number; error: string }[] = [];
	const remove: Rivet.cloud.CdnNamespaceAuthUser["user"][] = [];

	for (const [idx, user] of newUsers.entries()) {
		if (existingUsers.includes(user.user)) {
			// Update the user
			if (user.password.length > 1) {
				update.push(user);
			}
		} else {
			if (user.password.length < 1) {
				errors.push({
					idx,
					error: "Password must be at least 1 character long",
				});
			} else {
				// Create the user
				create.push(user);
			}
		}
	}

	for (const user of existingUsers) {
		if (!newUsers.find((u) => u.user === user)) {
			// Remove the user
			remove.push(user);
		}
	}

	return { update, create, errors, remove };
}

interface UseCdnManageAuthUsersFormHandlerProps {
	projectId: string;
	environmentId: string;
	userList: Rivet.cloud.CdnNamespaceAuthUser[];
	onSuccess?: () => void;
}

export function useCdnManageAuthUsersFormHandler({
	onSuccess,
	projectId,
	userList,
	environmentId,
}: UseCdnManageAuthUsersFormHandlerProps) {
	const { mutateAsync: updateUser } =
		useEnvironmentUpdateCdnAuthUserMutation();
	const { mutateAsync: removeUser } =
		useEnvironmentRemoveCdnAuthUserMutation();

	return useCallback<SubmitHandler>(
		async (values, form) => {
			const diff = computeUsersDiff(
				userList.map((user) => user.user),
				values.users,
			);
			if (diff.errors.length > 0) {
				for (const { idx, error } of diff.errors) {
					form.setError(`users.${idx}.password`, {
						type: "manual",
						message: error,
					});
				}
				return;
			}

			await Promise.all([
				...[...diff.update, ...diff.create].map((user) =>
					updateUser({
						projectId,
						environmentId,
						user: user.user,
						password: bcrypt.hashSync(user.password, SALT),
					}),
				),
				...diff.remove.map((user) =>
					removeUser({
						projectId,
						environmentId,
						user,
					}),
				),
			]);

			await queryClient.invalidateQueries(projectQueryOptions(projectId));
			await queryClient.invalidateQueries(
				projectEnvironmentQueryOptions({ projectId, environmentId }),
			);
			onSuccess?.();
		},
		[projectId, environmentId, onSuccess, removeUser, updateUser, userList],
	);
}
