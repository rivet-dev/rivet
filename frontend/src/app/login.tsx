"use client";
import { faGoogle, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
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
import { authClient, redirectToOrganization } from "@/lib/auth";

export function Login() {
	const navigate = useNavigate();
	const from = useSearch({ strict: false, select: (s) => s?.from as string });

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
			const result = await authClient.signIn.email({
				email,
				password,
			});

			if (result.error) {
				setError(result.error.message ?? "Invalid credentials");
				setIsLoading(false);
				return;
			}

			// Redirect to org page
			try {
				await redirectToOrganization(
					from ? { from } : {},
				);
			} catch (e) {
				// redirectToOrganization throws a redirect
				throw e;
			}

			// Fallback navigation if no redirect thrown
			await navigate({ to: from ?? "/", search: true });
		} catch (e) {
			// Re-throw redirect errors from TanStack Router
			if (e && typeof e === "object" && "to" in e) {
				throw e;
			}
			setError("An unexpected error occurred");
			setIsLoading(false);
		}
	};

	const handleGoogleSignIn = async () => {
		setError(null);
		setIsGoogleLoading(true);

		try {
			await authClient.signIn.social({
				provider: "google",
				callbackURL: from ?? "/",
			});
		} catch {
			setError("Failed to initiate Google sign-in");
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
						Enter your email below to login to your account.
					</CardDescription>
				</CardHeader>
				<form onSubmit={handleSubmit}>
					<CardContent className="grid gap-y-4">
						<div className="grid grid-cols-1 gap-x-4">
							<Button
								variant="outline"
								type="button"
								disabled={isGoogleLoading || isLoading}
								onClick={handleGoogleSignIn}
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
								autoComplete="current-password"
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
									"Sign in"
								)}
							</Button>
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
				</form>
			</Card>
		</motion.div>
	);
}
