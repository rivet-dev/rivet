import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { toast } from "sonner";
import { useInterval } from "@/components/hooks/use-interval";
import { formatDuration } from "@/components/lib/formatter";
import { RelativeTime } from "@/components/relative-time";
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
	const [retryUntil, setRetryUntil] = useState<Date | null>(null);

	useInterval(
		() => {
			setRetryUntil((prev) => {
				if (!prev || Date.now() >= prev.getTime()) return null;
				// New object with same timestamp forces RelativeTime's useMemo to recompute.
				return new Date(prev.getTime());
			});
		},
		retryUntil ? 1000 : null,
	);

	const handleResend = async () => {
		if (!email) return;
		setIsPending(true);
		try {
			let retryAfter: string | null = null;
			const result = await authClient.sendVerificationEmail(
				{ email },
				{
					onError(ctx) {
						retryAfter = ctx.response.headers.get("x-retry-after");
					},
				},
			);

			if (result.error) {
				if (result.error.status === 429) {
					const seconds = retryAfter
						? Number.parseInt(retryAfter, 10)
						: null;
					if (seconds && !Number.isNaN(seconds)) {
						setRetryUntil(new Date(Date.now() + seconds * 1000));
						toast.error(
							`Too many requests. Please try again in ${formatDuration(seconds * 1000, { showSeconds: true })}.`,
						);
					} else {
						toast.error("Too many requests. Please try again later.");
					}
				} else {
					toast.error(
						"Failed to resend verification email. Please try again.",
					);
				}
			} else {
				toast.success("Verification email sent. Check your inbox.");
			}
		} finally {
			setIsPending(false);
		}
	};

	const isRateLimited = retryUntil !== null;

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
							disabled={isRateLimited}
						>
							{isRateLimited && retryUntil ? (
								<>
									Retry <RelativeTime time={retryUntil} />
								</>
							) : (
								"Resend email"
							)}
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
