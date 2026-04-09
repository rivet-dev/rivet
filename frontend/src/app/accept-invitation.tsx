import { faGoogle, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery } from "@tanstack/react-query";
import { isRedirect, useNavigate, useSearch } from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { authClient, redirectToOrganization } from "@/lib/auth";

type InvitationDetails = {
	id: string;
	email: string;
	role: string;
	organizationName: string;
	status: string;
	expiresAt: string;
};

export function AcceptInvitation() {
	const navigate = useNavigate();
	const invitationId = useSearch({
		strict: false,
		select: (s) => s?.invitationId as string | undefined,
	});
	const token = useSearch({
		strict: false,
		select: (s) => s?.token as string | undefined,
	});
	const { data: session, isPending: sessionPending } =
		authClient.useSession();

	const resolvedInvitationId = invitationId ?? token;

	const {
		data: invitation,
		isPending: invitationPending,
		error: invitationError,
	} = useQuery({
		queryKey: ["invitation", resolvedInvitationId],
		queryFn: async () => {
			const result = await authClient.organization.getInvitation({
				query: { id: resolvedInvitationId! },
			});
			if (result.error || !result.data) {
				throw new Error(
					result.error?.message ??
						"This invitation link is invalid, expired, or has already been used.",
				);
			}
			const data = result.data as unknown as InvitationDetails;
			if (data.status !== "pending") {
				throw new Error(
					data.status === "accepted"
						? "This invitation has already been accepted."
						: "This invitation has expired or is no longer valid.",
				);
			}
			return data;
		},
		enabled: !!resolvedInvitationId,
		retry: false,
	});

	const acceptMutation = useMutation({
		mutationFn: async () => {
			const result = await authClient.organization.acceptInvitation({
				invitationId: resolvedInvitationId!,
			});
			if (result.error) {
				throw new Error(
					result.error.message ?? "Failed to accept invitation",
				);
			}
			const [error] = await attemptAsync(
				async () => await redirectToOrganization(),
			);
			if (error && isRedirect(error)) {
				navigate(error.options);
			}
		},
	});

	const rejectMutation = useMutation({
		mutationFn: async () => {
			await authClient.organization.rejectInvitation({
				invitationId: resolvedInvitationId!,
			});
		},
		onSuccess: () => navigate({ to: "/" }),
	});

	const handleGoogleSignIn = async () => {
		await authClient.signIn.social({
			provider: "google",
			callbackURL: window.location.href,
		});
	};

	if (sessionPending || invitationPending) {
		return (
			<div className="flex min-h-screen items-center justify-center bg-background">
				<p className="text-muted-foreground text-sm">Loading…</p>
			</div>
		);
	}

	if (!resolvedInvitationId || invitationError) {
		return (
			<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
				<Card className="w-full max-w-md">
					<CardHeader>
						<CardTitle>Invitation unavailable</CardTitle>
						<CardDescription>
							{invitationError?.message ??
								"No invitation ID found in this link."}
						</CardDescription>
					</CardHeader>
					<CardFooter>
						<Button
							variant="outline"
							onClick={() => navigate({ to: "/" })}
						>
							Go to dashboard
						</Button>
					</CardFooter>
				</Card>
			</div>
		);
	}

	if (!session) {
		return (
			<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
				<Card className="w-full max-w-md">
					<CardHeader>
						<CardTitle>You've been invited</CardTitle>
						<CardDescription>
							{invitation
								? `Sign in or create an account to join ${invitation.organizationName}.`
								: "Sign in or create an account to accept this invitation."}
						</CardDescription>
					</CardHeader>
					<CardContent className="grid gap-y-3">
						<Button onClick={handleGoogleSignIn}>
							<Icon icon={faGoogle} className="mr-2 size-4" />
							Continue with Google
						</Button>
						<Button
							variant="outline"
							onClick={() =>
								navigate({
									to: "/login",
									search: {
										from:
											window.location.pathname +
											window.location.search,
									},
								})
							}
						>
							Sign in with email
						</Button>
						<Button
							variant="ghost"
							onClick={() =>
								navigate({
									to: "/join",
									search: {
										from:
											window.location.pathname +
											window.location.search,
									},
								})
							}
						>
							Create a new account
						</Button>
					</CardContent>
				</Card>
			</div>
		);
	}

	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
			<Card className="w-full max-w-md">
				<CardHeader>
					<CardTitle>You've been invited</CardTitle>
					<CardDescription>
						{invitation
							? `You've been invited to join ${invitation.organizationName} as ${invitation.role}.`
							: "Accept the invitation to join the organization."}
					</CardDescription>
				</CardHeader>
				{acceptMutation.error && (
					<CardContent>
						<p className="text-sm text-destructive">
							{acceptMutation.error.message}
						</p>
					</CardContent>
				)}
				<CardFooter className="gap-2">
					<Button
						onClick={() => acceptMutation.mutate()}
						isLoading={acceptMutation.isPending}
					>
						Accept invitation
					</Button>
					<Button
						variant="outline"
						onClick={() => rejectMutation.mutate()}
						isLoading={rejectMutation.isPending}
					>
						Decline
					</Button>
				</CardFooter>
			</Card>
		</div>
	);
}
