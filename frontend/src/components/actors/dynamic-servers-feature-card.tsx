import { Link as RouterLink } from "@tanstack/react-router";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Link,
	Text,
} from "@/components";

export function DynamicServersFeatureCard() {
	return (
		<Card>
			<CardHeader>
				<CardTitle>Legacy Lobbies and Environments</CardTitle>
			</CardHeader>
			<CardContent>
				<Text>
					Dynamic servers and builds are the new way to manage your
					project servers. However, if you're looking for lobbies and
					namespaces, you can switch back to the old interface in the{" "}
					<Link asChild>
						<RouterLink to="/my-profile/features">
							User Settings
						</RouterLink>
					</Link>
					.
				</Text>
			</CardContent>
		</Card>
	);
}
