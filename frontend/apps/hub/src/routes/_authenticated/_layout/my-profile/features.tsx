import { type FeatureFlag, useFeatureFlag } from "@/hooks/use-feature-flag";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Flex,
	Strong,
	Switch,
	Text,
} from "@rivet-gg/components";
import { createFileRoute } from "@tanstack/react-router";
import { usePostHog } from "posthog-js/react";
import type { ReactNode } from "react";

interface FeatureCardProps {
	title: string;
	description: ReactNode;
	featureFlag: FeatureFlag;
	onChanged: (enabled: boolean) => void;
}

function FeatureCard({
	title,
	description,
	featureFlag,
	onChanged,
}: FeatureCardProps) {
	const isEnabled = useFeatureFlag(featureFlag);
	return (
		<Card>
			<CardHeader>
				<Flex justify="between">
					<CardTitle>{title}</CardTitle>
					<Switch
						data-feature-flag-name={featureFlag}
						checked={isEnabled}
						onCheckedChange={onChanged}
					/>
				</Flex>
			</CardHeader>
			<CardContent>{description}</CardContent>
		</Card>
	);
}

function MyProfileFeaturesRoute() {
	const posthog = usePostHog();

	return (
		<>
			<FeatureCard
				title="Servers and Builds"
				description={
					<Text>
						Servers and Builds are the new way for managing your
						project servers. They replace legacy namespaces and
						versions. Turn this feature off to use the legacy
						system. <Strong>Not recommended for new users.</Strong>
					</Text>
				}
				featureFlag="hub-dynamic-servers"
				onChanged={() => {
					// FIXME: use internal properties api
				}}
			/>
		</>
	);
}

export const Route = createFileRoute(
	"/_authenticated/_layout/my-profile/features",
)({
	component: MyProfileFeaturesRoute,
});
