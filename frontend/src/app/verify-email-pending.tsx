import { Link } from "@tanstack/react-router";
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

	const handleResend = async () => {
		if (!email) return;
		await authClient.sendVerificationEmail({ email });
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
						<Button variant="outline" onClick={handleResend}>
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
