import { isClerkAPIResponseError } from "@clerk/clerk-js";
import { useOrganization, useSignIn, useSignUp } from "@clerk/clerk-react";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { useEffect, useState } from "react";
import { useIsMounted } from "usehooks-ts";
import z from "zod";
import * as OrgSignUpForm from "@/app/forms/org-sign-up-form";
import { Logo } from "@/app/logo";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	toast,
} from "@/components";
import { clerk } from "@/lib/auth";

export const Route = createFileRoute("/onboarding/accept-invitation")({
	component: RouteComponent,
	validateSearch: zodValidator(
		z
			.object({
				__clerk_ticket: z.string().optional(),
				__clerk_status: z.string().optional(),
			})
			.optional(),
	),
});

function RouteComponent() {
	const search = Route.useSearch();
	const { organization } = useOrganization();

	if (
		search?.__clerk_status === "sign_up" &&
		search.__clerk_ticket &&
		!organization
	) {
		// display sign up flow
		return (
			<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
				<div className="flex flex-col items-center gap-6">
					<Logo className="h-10 mb-4" />
					<OrgSignUpFlow ticket={search.__clerk_ticket} />
				</div>
			</div>
		);
	}

	if (
		search?.__clerk_status === "sign_in" &&
		search.__clerk_ticket &&
		!organization
	) {
		// complete sign in flow
		return (
			<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
				<div className="flex flex-col items-center gap-6">
					<Logo className="h-10 mb-4" />
					<OrgSignInFlow ticket={search.__clerk_ticket} />
				</div>
			</div>
		);
	}

	if (
		search?.__clerk_status === "sign_in" &&
		search.__clerk_ticket &&
		organization
	) {
		// if we get here, the user is already signed in but maybe not to the right org
		return <AcceptInvitation ticket={search.__clerk_ticket} />;
	}

	if (search?.__clerk_status === "complete") {
		// if we get here, the user is already signed in
		return <CompleteFlow />;
	}

	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6">
				<Logo className="h-10 mb-4" />
				<Card className="w-full sm:w-96">
					<CardHeader>
						<CardTitle>Invalid Invitation</CardTitle>
						<CardDescription>
							The invitation link is invalid. Please check the
							link or contact support.
						</CardDescription>
					</CardHeader>
				</Card>
			</div>
		</div>
	);
}

function OrgSignUpFlow({ ticket }: { ticket: string }) {
	const { signUp, setActive: setActiveSignUp } = useSignUp();
	const navigate = useNavigate();

	return (
		<OrgSignUpForm.Form
			defaultValues={{ password: "" }}
			onSubmit={async ({ password }, form) => {
				try {
					const signUpAttempt = await signUp?.create({
						strategy: "ticket",
						ticket,
						password,
					});

					if (signUpAttempt?.status === "complete") {
						await setActiveSignUp?.({
							session: signUpAttempt.createdSessionId,
						});
						await navigate({ to: "/" });
					} else {
						console.error(
							"Sign up attempt not complete",
							signUpAttempt,
						);
						toast.error(
							"An error occurred during sign up. Please try again.",
						);
					}
				} catch (e) {
					if (isClerkAPIResponseError(e)) {
						for (const error of e.errors) {
							form.setError(
								(error.meta?.paramName || "root") as "root",
								{
									message: error.longMessage,
								},
							);
						}
					} else {
						toast.error(
							"An unknown error occurred. Please try again.",
						);
					}
				}
			}}
		>
			<Card className="w-full sm:w-96">
				<CardHeader>
					<CardTitle>Welcome!</CardTitle>
					<CardDescription>
						Please set a password for your new account.
					</CardDescription>
				</CardHeader>
				<CardContent>
					<div>
						<OrgSignUpForm.Password className="mb-4" />
					</div>
				</CardContent>
				<CardFooter>
					<OrgSignUpForm.Submit className="w-full">
						Continue
					</OrgSignUpForm.Submit>
				</CardFooter>
			</Card>
		</OrgSignUpForm.Form>
	);
}

function OrgSignInFlow({ ticket }: { ticket: string }) {
	const { organization } = useOrganization();
	const { signIn, setActive: setActiveSignIn } = useSignIn();
	const isMounted = useIsMounted();
	const navigate = useNavigate();

	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		async function signInWithTicket() {
			const signInAttempt = await signIn?.create({
				strategy: "ticket",
				ticket,
			});

			// If the sign-in was successful, set the session to active
			if (signInAttempt?.status === "complete") {
				await setActiveSignIn?.({
					session: signInAttempt?.createdSessionId,
				});
				await navigate({ to: "/" });
			} else {
				// If the sign-in attempt is not complete, check why.
				// User may need to complete further steps.
				console.error(JSON.stringify(signInAttempt, null, 2));
			}
		}

		signInWithTicket().catch((e) => {
			if (isClerkAPIResponseError(e)) {
				setError(e.message);
			} else {
				setError("An unknown error occurred. Please try again.");
			}
		});
	}, [isMounted]);

	return (
		<Card className="w-full sm:w-96">
			<CardHeader>
				<CardTitle>Welcome back!</CardTitle>
				<CardDescription>
					You are signing in to {organization?.name || "your account"}
					.
				</CardDescription>
			</CardHeader>
			{error && (
				<CardContent>
					<div className="text-destructive">{error}</div>
				</CardContent>
			)}
		</Card>
	);
}

function CompleteFlow() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6">
				<Logo className="h-10 mb-4" />
				<Card className="w-full sm:w-96">
					<CardHeader>
						<CardTitle>Invitation Accepted</CardTitle>
						<CardDescription>
							You have successfully accepted the invitation. You
							can now proceed to the dashboard.
						</CardDescription>
					</CardHeader>
					<CardFooter>
						<Button asChild>
							<Link to="/">Go Home</Link>
						</Button>
					</CardFooter>
				</Card>
			</div>
		</div>
	);
}

function AcceptInvitation({ ticket }: { ticket: string }) {
	useEffect(() => {
		clerk.getFapiClient().request({
			path: "/tickets/accept",
			search: { ticket },
		});
	}, [ticket]);
	return <CompleteFlow />;
}
