import { ls } from "@/lib/ls";
import {
	Button,
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	Text,
} from "@rivet-gg/components";
import { Icon, faFlagCheckered } from "@rivet-gg/icons";
import { useQueryClient } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";

export const NotFoundComponent = () => {
	const router = useRouter();
	const queryClient = useQueryClient();

	return (
		<Card w="full">
			<CardHeader>
				<CardTitle className="flex gap-2">
					<Icon icon={faFlagCheckered} />
					Wrong direction!
				</CardTitle>
			</CardHeader>
			<CardContent>
				<Text>This page does not exists!</Text>
			</CardContent>
			<CardFooter>
				<Button
					onClick={() => {
						queryClient.invalidateQueries();
						ls.clear();
						router.navigate({ to: "/" });
					}}
				>
					Homepage
				</Button>
			</CardFooter>
		</Card>
	);
};
