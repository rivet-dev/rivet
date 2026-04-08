import { Link } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { useState } from "react";
import {
	EmailField,
	Form,
	RootError,
	Submit,
	type SubmitHandler,
} from "@/components/forms/forgot-password-form";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { authClient } from "@/lib/auth";

export function ForgotPassword() {
	const [sent, setSent] = useState(false);

	const handleSubmit: SubmitHandler = async ({ email }, form) => {
		const result = await authClient.requestPasswordReset({
			email,
			redirectTo: `${window.location.origin}/reset-password`,
		});

		if (result.error) {
			form.setError("root", {
				message: result.error.message ?? "Failed to send reset email",
			});
			return;
		}

		setSent(true);
	};

	if (sent) {
		return (
			<motion.div
				className="grid w-full grow items-center px-4"
				initial={{ opacity: 0, y: 10 }}
				animate={{ opacity: 1, y: 0 }}
				exit={{ opacity: 0, y: 10 }}
			>
				<Card className="w-full max-w-md grow mx-auto">
					<CardHeader>
						<CardTitle>Check your email</CardTitle>
						<CardDescription>
							We sent a password reset link. Check your inbox and
							follow the instructions.
						</CardDescription>
					</CardHeader>
					<CardFooter>
						<Button
							variant="link"
							className="text-primary-foreground"
							size="sm"
							asChild
						>
							<Link to="/login">Back to sign in</Link>
						</Button>
					</CardFooter>
				</Card>
			</motion.div>
		);
	}

	return (
		<motion.div
			className="grid w-full grow items-center px-4"
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			exit={{ opacity: 0, y: 10 }}
		>
			<Card className="w-full max-w-md grow mx-auto">
				<CardHeader>
					<CardTitle>Reset your password</CardTitle>
					<CardDescription>
						Enter your email and we'll send you a reset link.
					</CardDescription>
				</CardHeader>
				<Form defaultValues={{ email: "" }} onSubmit={handleSubmit}>
					<CardContent className="grid gap-y-4">
						<EmailField />
						<RootError />
					</CardContent>
					<CardFooter>
						<div className="grid w-full gap-y-4">
							<Submit allowPristine>Send reset link</Submit>
							<Button
								variant="link"
								className="text-primary-foreground"
								size="sm"
								asChild
							>
								<Link to="/login">Back to sign in</Link>
							</Button>
						</div>
					</CardFooter>
				</Form>
			</Card>
		</motion.div>
	);
}
