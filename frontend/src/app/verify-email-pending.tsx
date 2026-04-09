import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { toast } from "sonner";
import { formatDuration } from "@/components/lib/formatter";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { authClient } from "@/lib/auth";

export function VerifyEmailPending() {
	const { data: session } = authClient.useSession();
	const email = session?.user.email;
	const [isPending, setIsPending] = useState(false);

	const handleResend = async () => {
		if (!email) return;
		setIsPending(true);

		let retryAfter: string | null = null;
		const result = await authClient.sendVerificationEmail(
			{ email },
			{
				onError(ctx) {
					retryAfter = ctx.response.headers.get("x-retry-after");
				},
			},
		);

		setIsPending(false);

		if (result.error) {
			if (result.error.status === 429) {
				const seconds = retryAfter ? Number.parseInt(retryAfter, 10) : null;
				const wait =
					seconds && !Number.isNaN(seconds)
						? formatDuration(seconds * 1000, { showSeconds: true })
						: "a moment";
				toast.error(`Too many requests. Please try again in ${wait}.`);
			} else {
				toast.error("Failed to resend verification email. Please try again.");
			}
		} else {
			toast.success("Verification email sent. Check your inbox.");
		}
	};

	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
			<Card className="w-full max-w-md">
				<CardHeader>
					<CardTitle>Check your email</CardTitle>
					<CardDescription>
						We sent a verification link to{" "}
						{email ? (
							<span className="font-medium text-foreground">
								{email}
							</span>
						) : (
							"your email address"
						)}
						. Click it to activate your account.
					</CardDescription>
				</CardHeader>
				<CardFooter>
					<div className="grid w-full gap-y-4">
						<Button
							variant="outline"
							onClick={handleResend}
							isLoading={isPending}
						>
							Resend email
						</Button>
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
			</Card>
		</div>
	);
}
