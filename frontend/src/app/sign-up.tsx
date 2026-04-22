"use client";
import { faGoogle, Icon } from "@rivet-gg/icons";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { useRef, useState } from "react";
import {
	EmailField,
	Form,
	NameField,
	PasswordField,
	RootError,
	Submit,
	type SubmitHandler,
} from "@/components/forms/sign-up-form";
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
import { authClient } from "@/lib/auth";
import { cloudEnv } from "@/lib/env";
import { features } from "@/lib/features";
import { LoginWithGoogle } from "./login";

export function SignUp() {
	const navigate = useNavigate();
	const [turnstileToken, setTurnstileToken] = useState<string | null>(null);
	const turnstileSiteKey = cloudEnv().VITE_APP_TURNSTILE_SITE_KEY;
	const handleSubmit: SubmitHandler = async (
		{ name, email, password },
		form,
	) => {
		if (features.captcha && !turnstileToken) {
			form.setError("root", {
				message:
					"Captcha verification is still loading, please try again",
			});
			return;
		}

		const result = await authClient.signUp.email(
			{
				email,
				password,
				name,
				callbackURL: `${window.location.origin}/?emailVerified=1`,
			},
			features.captcha && turnstileToken
				? { headers: { "x-captcha-response": turnstileToken } }
				: undefined,
		);

		if (result.error) {
			form.setError("root", {
				message: result.error.message ?? "Sign up failed",
			});
			return;
		}

		setTurnstileToken(null);
		navigate({ to: "/verify-email-pending", search: { email } });
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
						Create your account to get started.
					</CardDescription>
				</CardHeader>
				<Form
					defaultValues={{ name: "", email: "", password: "" }}
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
							<NameField />
							<EmailField />
							<PasswordField />
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
							<Submit allowPristine>Continue</Submit>
							<Button
								variant="link"
								className="text-primary-foreground"
								size="sm"
								asChild
							>
								<Link to="/login">
									Already have an account? Sign in
								</Link>
							</Button>
						</div>
					</CardFooter>
				</Form>
			</Card>
		</motion.div>
	);
}
