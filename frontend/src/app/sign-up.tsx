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
import { authClient, redirectToOrganization } from "@/lib/auth";

export function SignUp() {
	const navigate = useNavigate();
	const from = useSearch({ strict: false, select: (s) => s?.from as string });

	const handleSubmit: SubmitHandler = async (
		{ name, email, password },
		form,
	) => {
		const result = await authClient.signUp.email({ email, password, name });

		if (result.error) {
			form.setError("root", {
				message: result.error.message ?? "Sign up failed",
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

	const handleGoogleSignUp = async () => {
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
						Create your account to get started.
					</CardDescription>
				</CardHeader>
				<Form
					defaultValues={{ name: "", email: "", password: "" }}
					onSubmit={handleSubmit}
				>
					<CardContent className="grid gap-y-4">
						<div className="grid grid-cols-1 gap-x-4">
							<Button
								variant="outline"
								type="button"
								onClick={handleGoogleSignUp}
							>
								<Icon icon={faGoogle} className="mr-2 size-4" />
								Google
							</Button>
						</div>
						<p className="flex items-center gap-x-3 text-sm text-muted-foreground before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border">
							or
						</p>
						<NameField />
						<EmailField />
						<PasswordField />
						<RootError />
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
