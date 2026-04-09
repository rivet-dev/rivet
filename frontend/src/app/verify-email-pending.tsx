import { useMutation } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { toast } from "sonner";
import { useTimeout } from "usehooks-ts";
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
import { isAuthError, isDate } from "@/lib/utils";

export function VerifyEmailPending() {
	const { data: session } = authClient.useSession();
	const email = session?.user.email;

	const { mutate, isPending, isError, error, reset } = useMutation({
		mutationFn: async () => {
			if (!email) return;
			let retryAfter: Date | null = null;
			const result = await authClient.sendVerificationEmail({
				email,
				callbackURL: window.location.origin,
				fetchOptions: {
					onError: async (context) => {
						const { response } = context;
						if (response.status === 429) {
							const retryAfterHeader =
								response.headers.get("X-Retry-After");
							retryAfter = retryAfterHeader
								? new Date(
										Date.now() +
											Number.parseInt(
												retryAfterHeader,
												10,
											) *
												1000,
									)
								: null;
						}
					},
				},
			});

			if (result.error) {
				throw { ...result.error, retryAfter };
			}
			return result.data;
		},
		onSuccess: () => {
			toast.success("Verification email resent");
		},
	});

	const retryAfter =
		isError && isAuthError(error) && isDate(error.retryAfter)
			? error.retryAfter
			: null;
	const rateLimited = isError && isAuthError(error) && error.status === 429;

	useTimeout(
		() => {
			reset();
		},
		retryAfter ? retryAfter.getTime() - Date.now() : null,
	);

	const handleResend = () => {
		if (!email) return;
		mutate();
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
							disabled={rateLimited}
						>
							{rateLimited && retryAfter ? (
								<>
									Retry <RelativeTime time={retryAfter} />
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
