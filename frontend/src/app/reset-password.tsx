import { isRedirect, Link, useNavigate, useSearch } from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { motion } from "framer-motion";
import { toast } from "sonner";
import {
	ConfirmPasswordField,
	Form,
	NewPasswordField,
	RootError,
	Submit,
	type SubmitHandler,
} from "@/components/forms/reset-password-form";
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

export function ResetPassword() {
	const navigate = useNavigate();
	const token = useSearch({
		strict: false,
		select: (s) => s?.token as string | undefined,
	});

	const handleSubmit: SubmitHandler = async ({ newPassword }, form) => {
		if (!token) {
			form.setError("root", {
				message:
					"Missing reset token. Please use the link from your email.",
			});
			return;
		}

		const result = await authClient.resetPassword({ newPassword, token });

		if (result.error) {
			form.setError("root", {
				message: result.error.message ?? "Failed to reset password",
			});
			return;
		}

		toast.success("Password updated. You can now sign in.");

		const [error] = await attemptAsync(
			async () => await redirectToOrganization(),
		);

		if (error && isRedirect(error)) {
			navigate(error.options);
			return;
		}

		navigate({ to: "/login" });
	};

	if (!token) {
		return (
			<motion.div
				className="grid w-full grow items-center px-4"
				initial={{ opacity: 0, y: 10 }}
				animate={{ opacity: 1, y: 0 }}
				exit={{ opacity: 0, y: 10 }}
			>
				<Card className="w-full max-w-md grow mx-auto">
					<CardHeader>
						<CardTitle>Link invalid or expired</CardTitle>
						<CardDescription>
							This password reset link is invalid or has expired.
						</CardDescription>
					</CardHeader>
					<CardFooter>
						<div className="grid w-full gap-y-4">
							<Button asChild>
								<Link to="/forgot-password">
									Request a new link
								</Link>
							</Button>
						</div>
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
					<CardTitle>Choose a new password</CardTitle>
					<CardDescription>
						Enter a new password for your account.
					</CardDescription>
				</CardHeader>
				<Form
					defaultValues={{ newPassword: "", confirmPassword: "" }}
					onSubmit={handleSubmit}
				>
					<CardContent className="grid gap-y-4">
						<NewPasswordField />
						<ConfirmPasswordField />
						<RootError />
					</CardContent>
					<CardFooter>
						<Submit allowPristine className="w-full">
							Set new password
						</Submit>
					</CardFooter>
				</Form>
			</Card>
		</motion.div>
	);
}
