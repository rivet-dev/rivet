"use client";
import { faGoogle, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { Link, useNavigate } from "@tanstack/react-router";
import { motion } from "framer-motion";
import { type FormEvent, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { authClient } from "@/lib/auth";

export function SignUp() {
	const navigate = useNavigate();

	const [name, setName] = useState("");
	const [email, setEmail] = useState("");
	const [password, setPassword] = useState("");
	const [error, setError] = useState<string | null>(null);
	const [isLoading, setIsLoading] = useState(false);
	const [isGoogleLoading, setIsGoogleLoading] = useState(false);

	const handleSubmit = async (e: FormEvent) => {
		e.preventDefault();
		setError(null);
		setIsLoading(true);

		try {
			const result = await authClient.signUp.email({
				email,
				password,
				name,
			});

			if (result.error) {
				setError(result.error.message ?? "Sign up failed");
				setIsLoading(false);
				return;
			}

			// On success, redirect to onboarding
			await navigate({ to: "/onboarding/choose-organization" });
		} catch (e) {
			// Re-throw redirect errors from TanStack Router
			if (e && typeof e === "object" && "to" in e) {
				throw e;
			}
			setError("An unexpected error occurred");
			setIsLoading(false);
		}
	};

	const handleGoogleSignUp = async () => {
		setError(null);
		setIsGoogleLoading(true);

		try {
			await authClient.signIn.social({
				provider: "google",
				callbackURL: "/onboarding/choose-organization",
			});
		} catch {
			setError("Failed to initiate Google sign-up");
			setIsGoogleLoading(false);
		}
	};

	return (
		<motion.div
			className="grid w-full grow items-center px-4 sm:justify-center"
			initial={{ opacity: 0, y: 10 }}
			animate={{ opacity: 1, y: 0 }}
			exit={{ opacity: 0, y: 10 }}
		>
			<Card className="w-full sm:w-96">
				<CardHeader>
					<CardTitle>Welcome!</CardTitle>
					<CardDescription>
						Create your account to get started.
					</CardDescription>
				</CardHeader>
				<form onSubmit={handleSubmit}>
					<CardContent className="grid gap-y-4">
						<div className="grid grid-cols-1 gap-x-4">
							<Button
								variant="outline"
								type="button"
								disabled={isGoogleLoading || isLoading}
								onClick={handleGoogleSignUp}
							>
								{isGoogleLoading ? (
									<Icon
										icon={faSpinnerThird}
										className="size-4 animate-spin"
									/>
								) : (
									<>
										<Icon
											icon={faGoogle}
											className="mr-2 size-4"
										/>
										Google
									</>
								)}
							</Button>
						</div>
						<p className="flex items-center gap-x-3 text-sm text-muted-foreground before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border">
							or
						</p>
						<div className="space-y-2">
							<Label htmlFor="name">Name</Label>
							<Input
								id="name"
								type="text"
								required
								placeholder="Your name"
								value={name}
								onChange={(e) => setName(e.target.value)}
								disabled={isLoading}
							/>
						</div>
						<div className="space-y-2">
							<Label htmlFor="email">Email address</Label>
							<Input
								id="email"
								type="email"
								required
								placeholder="you@company.com"
								value={email}
								onChange={(e) => setEmail(e.target.value)}
								disabled={isLoading}
							/>
						</div>
						<div className="space-y-2">
							<Label htmlFor="password">Password</Label>
							<Input
								id="password"
								type="password"
								required
								placeholder="Your password"
								autoComplete="new-password"
								value={password}
								onChange={(e) => setPassword(e.target.value)}
								disabled={isLoading}
							/>
						</div>
						{error ? (
							<p className="text-sm text-destructive">{error}</p>
						) : null}
					</CardContent>
					<CardFooter>
						<div className="grid w-full gap-y-4">
							<Button
								type="submit"
								disabled={isLoading || isGoogleLoading}
							>
								{isLoading ? (
									<Icon
										icon={faSpinnerThird}
										className="size-4 animate-spin"
									/>
								) : (
									"Continue"
								)}
							</Button>
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
				</form>
			</Card>
		</motion.div>
	);
}
