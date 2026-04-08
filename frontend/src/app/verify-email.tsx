import { isRedirect, Link, useNavigate, useSearch } from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { authClient, redirectToOrganization } from "@/lib/auth";

type Status = "loading" | "success" | "error";

export function VerifyEmail() {
	const navigate = useNavigate();
	const token = useSearch({
		strict: false,
		select: (s) => s?.token as string | undefined,
	});
	const [status, setStatus] = useState<Status>("loading");

	useEffect(() => {
		if (!token) {
			setStatus("error");
			return;
		}

		authClient.verifyEmail({ query: { token } }).then(async (result) => {
			if (result.error) {
				setStatus("error");
				return;
			}

			setStatus("success");

			const [error] = await attemptAsync(
				async () => await redirectToOrganization(),
			);

			if (error && isRedirect(error)) {
				navigate(error.options);
			}
		});
	}, [token, navigate]);

	if (status === "loading") {
		return (
			<div className="flex min-h-screen items-center justify-center bg-background">
				<p className="text-muted-foreground text-sm">Verifying your email…</p>
			</div>
		);
	}

	if (status === "success") {
		return (
			<div className="flex min-h-screen items-center justify-center bg-background">
				<p className="text-muted-foreground text-sm">Email verified! Redirecting…</p>
			</div>
		);
	}

	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
			<Card className="w-full max-w-md">
				<CardHeader>
					<CardTitle>Link invalid or expired</CardTitle>
					<CardDescription>
						This verification link is invalid or has expired.
					</CardDescription>
				</CardHeader>
				<CardFooter>
					<div className="grid w-full gap-y-4">
						<Button asChild>
							<Link to="/join">Create a new account</Link>
						</Button>
						<Button variant="outline" asChild>
							<Link to="/login">Back to sign in</Link>
						</Button>
					</div>
				</CardFooter>
			</Card>
		</div>
	);
}
