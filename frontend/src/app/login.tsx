"use client";
import { faGoogle, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import {
	isRedirect,
	Link,
	useNavigate,
} from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { motion } from "framer-motion";
import { useState } from "react";
import { useFormContext } from "react-hook-form";
import {
	EmailField,
	Form,
	PasswordField,
	RootError,
	Submit,
	type SubmitHandler,
} from "@/components/forms/login-form";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { TurnstileWidget } from "@/components/ui/turnstile";
import { authClient, redirectToOrganization } from "@/lib/auth";
import { cloudEnv } from "@/lib/env";
import { features } from "@/lib/features";
import { isAuthError } from "@/lib/utils";

export function Login() {
	const navigate = useNavigate();
	const [turnstileToken, setTurnstileToken] = useState<string | null>(null);
	const turnstileSiteKey = cloudEnv().VITE_APP_TURNSTILE_SITE_KEY;

	const handleSubmit: SubmitHandler = async ({ email, password }, form) => {
		if (features.captcha && !turnstileToken) {
			form.setError("root", {
				message:
					"Captcha verification is still loading, please try again",
			});
			return;
		}

		const result = await authClient.signIn.email(
			{ email, password },
			features.captcha && turnstileToken
				? { headers: { "x-captcha-response": turnstileToken } }
				: undefined,
		);

		if (result.error) {
			form.setError("root", {
				message: result.error.message ?? "Invalid credentials",
			});
			return;
		}

		if (result.data?.user.emailVerified === false) {
			return navigate({ to: "/verify-email-pending", search: { email } });
		}

		setTurnstileToken(null);

		const [error] = await attemptAsync(
			async () => await redirectToOrganization(),
		);

		if (error && isRedirect(error)) {
			return navigate(error.options);
		}
	};

	return (
		<motion.div
			className="grid w-full grow items-center px-4"
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			exit={{ opacity: 0, y: 10 }}
		>
			<Card className="w-full max-w-[21.75rem] grow mx-auto">
				<CardHeader>
					<CardTitle>Welcome!</CardTitle>
					<CardDescription>
						Enter your email below to login to your account.
					</CardDescription>
				</CardHeader>
				<Form
					defaultValues={{ email: "", password: "" }}
					onSubmit={handleSubmit}
				>
					<CardContent className="gap-2 flex flex-col">
						<div className="grid gap-y-4">
							<div className="grid grid-cols-1 gap-x-4">
								<LoginWithGoogle />
							</div>
							<p className="flex items-center gap-x-3 text-sm text-muted-foreground before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border">
								or
							</p>
							<EmailField />
							<PasswordField />
							<div className="flex justify-end">
								<Link
									to="/forgot-password"
									className="text-xs text-muted-foreground underline-offset-4 hover:underline"
								>
									Forgot password?
								</Link>
							</div>
							<RootError />
						</div>
						{features.captcha && turnstileSiteKey && (
							<TurnstileWidget
								siteKey={turnstileSiteKey}
								onSuccess={setTurnstileToken}
								onExpire={() => setTurnstileToken(null)}
								onError={() => setTurnstileToken(null)}
								onTimeout={() => setTurnstileToken(null)}
							/>
						)}
					</CardContent>
					<CardFooter>
						<div className="grid w-full gap-y-4">
							<Submit allowPristine>Sign in</Submit>
							<Button
								variant="link"
								className="text-primary-foreground"
								size="sm"
								asChild
							>
								<Link to="/join">
									Don&apos;t have an account? Sign up
								</Link>
							</Button>
						</div>
					</CardFooter>
				</Form>
			</Card>
		</motion.div>
	);
}

export function LoginWithGoogle() {
	const form = useFormContext();

	const { isPending, mutate } = useMutation({
		mutationFn: async () => {
			return authClient.signIn.social({
				provider: "google",
				callbackURL: `${window.location.origin}/`,
			});
		},
		onSettled(response) {
			if (isAuthError(response?.error)) {
				form.setError("root", {
					message: response.error.message,
				});
			}
		},
	});

	return (
		<Button
			variant="outline"
			onClick={() => mutate()}
			type="button"
			isLoading={isPending}
		>
			<Icon icon={faGoogle} className="mr-2 size-4" />
			Sign in with Google
		</Button>
	);
}
