import { AccountDeletionCard } from "@/domains/user/components/account-deletion-card";
import { UserAvatarSettingsCard } from "@/domains/user/components/user-avatar-settings-card";
import { UserNameSettingsCard } from "@/domains/user/components/user-name-settings-card";
import { createFileRoute } from "@tanstack/react-router";

function MyProfileIndexRoute() {
	return (
		<>
			<UserNameSettingsCard />
			<UserAvatarSettingsCard />
			<AccountDeletionCard />
		</>
	);
}

export const Route = createFileRoute("/_authenticated/_layout/my-profile/")({
	component: MyProfileIndexRoute,
});
