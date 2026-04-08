"use client";
import { faGoogle, Icon } from "@rivet-gg/icons";
import {
	isRedirect,
	Link,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { motion } from "framer-motion";
import { useState } from "react";
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

export function Login() {
	const navigate = useNavigate();
	const from = useSearch({ strict: false, select: (s) => s?.from as string });
	const [turnstileToken, setTurnstileToken] = useState<string | null>(null);

	const handleSubmit: SubmitHandler = async ({ email, password }, form) => {
		if (features.captcha && !turnstileToken) {
			form.setError("root", {
				message: "Captcha verification is still loading, please try again",
			});
			return;
		}

		const result = await authClient.signIn.email(
			{ email, password },
			features.captcha && turnstileToken
				? { headers: { "x-captcha-response": turnstileToken } }
				: undefined,
		);

		setTurnstileToken(null);

		if (result.error) {
			form.setError("root", {
				message: result.error.message ?? "Invalid credentials",
			});
			return;
		}

		const [error] = await attemptAsync(
			async () => await redirectToOrganization(),
		);

		if (error && isRedirect(error)) {
			return navigate(error.options);
		}
	};

	const handleGoogleSignIn = async () => {
		await authClient.signIn.social({
			provider: "google",
			callbackURL: from ?? "/",
		});
	};

	return (
		<motion.div
			className="grid w-full grow items-center px-4"
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			exit={{ opacity: 0, y: 10 }}
		>
			<Card className="w-full max-w-md grow mx-auto">
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
					<CardContent className="grid gap-y-4">
						<div className="grid grid-cols-1 gap-x-4">
							<Button
								variant="outline"
								type="button"
								onClick={handleGoogleSignIn}
							>
								<Icon icon={faGoogle} className="mr-2 size-4" />
								Google
							</Button>
						</div>
						<p className="flex items-center gap-x-3 text-sm text-muted-foreground before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border">
							or
						</p>
						<EmailField />
						<PasswordField />
						<RootError />
						{features.captcha && (
							<TurnstileWidget
								siteKey={cloudEnv().VITE_APP_TURNSTILE_SITE_KEY!}
								onSuccess={setTurnstileToken}
								onExpire={() => setTurnstileToken(null)}
								onError={() => setTurnstileToken(null)}
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
