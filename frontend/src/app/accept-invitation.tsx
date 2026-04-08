import { faGoogle, Icon } from "@rivet-gg/icons";
import { isRedirect, useNavigate, useSearch } from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { useEffect, useState } from "react";
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

type Status = "loading" | "ready" | "accepting" | "error";

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
	const [status, setStatus] = useState<Status>("loading");
	const [errorMessage, setErrorMessage] = useState<string | null>(null);

	// The invitation ID may arrive as ?invitationId= or ?token= depending on server config.
	const resolvedInvitationId = invitationId ?? token;

	useEffect(() => {
		if (sessionPending) return;
		if (!resolvedInvitationId) {
			setStatus("error");
			setErrorMessage("No invitation ID found in this link.");
			return;
		}
		setStatus("ready");
	}, [sessionPending, resolvedInvitationId]);

	const handleAccept = async () => {
		if (!resolvedInvitationId) return;
		setStatus("accepting");

		const result = await authClient.organization.acceptInvitation({
			invitationId: resolvedInvitationId,
		});

		if (result.error) {
			setStatus("error");
			setErrorMessage(
				result.error.message ?? "Failed to accept invitation",
			);
			return;
		}

		const [error] = await attemptAsync(
			async () => await redirectToOrganization(),
		);

		if (error && isRedirect(error)) {
			navigate(error.options);
		}
	};

	const handleReject = async () => {
		if (!resolvedInvitationId) return;
		await authClient.organization.rejectInvitation({
			invitationId: resolvedInvitationId,
		});
		navigate({ to: "/" });
	};

	const handleGoogleSignIn = async () => {
		await authClient.signIn.social({
			provider: "google",
			callbackURL: window.location.href,
		});
	};

	if (sessionPending || status === "loading") {
		return (
			<div className="flex min-h-screen items-center justify-center bg-background">
				<p className="text-muted-foreground text-sm">Loading…</p>
			</div>
		);
	}

	if (status === "error") {
		return (
			<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
				<Card className="w-full max-w-md">
					<CardHeader>
						<CardTitle>Invitation unavailable</CardTitle>
						<CardDescription>
							{errorMessage ??
								"This invitation link is invalid, expired, or has already been used."}
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
							Sign in or create an account to accept this
							invitation.
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
						Accept the invitation to join the organization.
					</CardDescription>
				</CardHeader>
				<CardFooter className="gap-2">
					<Button
						onClick={handleAccept}
						isLoading={status === "accepting"}
					>
						Accept invitation
					</Button>
					<Button variant="outline" onClick={handleReject}>
						Decline
					</Button>
				</CardFooter>
			</Card>
		</div>
	);
}
