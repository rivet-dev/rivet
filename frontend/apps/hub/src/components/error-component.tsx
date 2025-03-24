import { ls } from "@/lib/ls";
import { hasMethod, isRivetError } from "@/lib/utils";
import {
	Button,
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	Code,
	Text,
} from "@rivet-gg/components";
import { PageLayout } from "@rivet-gg/components/layout";
import { Icon, faBomb, faLock } from "@rivet-gg/icons";
import * as Sentry from "@sentry/react";
import {
	useQueryClient,
	useQueryErrorResetBoundary,
} from "@tanstack/react-query";
import {
	type ErrorComponentProps,
	isNotFound,
	useRouter,
} from "@tanstack/react-router";
import { useEffect } from "react";
import { NetworkIssueError } from "./network-issue-error";
import { NotFoundComponent } from "./not-found-component";

export const ErrorComponent = ({
	error,
	reset,
}: Partial<ErrorComponentProps>) => {
	const router = useRouter();
	const queryClient = useQueryClient();
	const queryErrorResetBoundary = useQueryErrorResetBoundary();

	useEffect(() => {
		if (isNotFound(error)) {
			return;
		}
		console.dir(error);
		if (error) {
			Sentry.captureException(error);
		}

		// Reset the query error boundary
		queryErrorResetBoundary.reset();
	}, [error, queryErrorResetBoundary]);

	if (isRivetError(error)) {
		if (
			error.statusCode === 403 &&
			error.body.code === "GROUP_NOT_MEMBER"
		) {
			return (
				<Card>
					<CardHeader>
						<CardTitle className="flex gap-2">
							<Icon icon={faLock} />
							Unauthorized
						</CardTitle>
					</CardHeader>
					<CardContent>
						<Text>You are not a member of this group.</Text>
					</CardContent>
					<CardFooter>
						<Button
							onClick={() => {
								ls.clear();
								router.navigate({ to: "/" });
							}}
						>
							Go Home
						</Button>
					</CardFooter>
				</Card>
			);
		}
		if (error.statusCode === 404) {
			return <NotFoundComponent />;
		}
	} else if (!error) {
		return <NotFoundComponent />;
	} else if ("statusCode" in error && "body" in error) {
		return <NetworkIssueError />;
	}

	return (
		<Card>
			<CardHeader>
				<CardTitle className="flex gap-2">
					<Icon icon={faBomb} />
					Uh, oh!
				</CardTitle>
			</CardHeader>
			<CardContent>
				<Text>Something went wrong!</Text>
				<Code>
					{hasMethod(error, "toString")
						? (error.toString() as string)
						: JSON.stringify(error)}
				</Code>
			</CardContent>
			<CardFooter>
				<Button
					onClick={() => {
						router.invalidate();
						ls.clear();
						queryClient.resetQueries();
						queryClient.invalidateQueries();
						reset?.();
					}}
				>
					Retry
				</Button>
			</CardFooter>
		</Card>
	);
};

export function LayoutedErrorComponent(props: ErrorComponentProps) {
	return (
		<PageLayout.Root>
			<ErrorComponent {...props} />
		</PageLayout.Root>
	);
}
