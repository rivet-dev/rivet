import { AuthCard } from "@/components/auth-card";
import * as LoginForm from "@/domains/auth/forms/login-form";
import {
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	Flex,
	Link,
	MutedText,
} from "@rivet-gg/components";

interface EmailStepProps {
	onSubmit: LoginForm.SubmitHandler;
}

export const EmailStep = ({ onSubmit }: EmailStepProps) => {
	return (
		<LoginForm.Form defaultValues={{ email: "" }} onSubmit={onSubmit}>
			<AuthCard>
				<CardHeader>
					<CardTitle>Login</CardTitle>
					<CardDescription>
						Enter your email below to login to your account.
					</CardDescription>
				</CardHeader>
				<CardContent>
					<Flex gap="4" direction="col">
						<LoginForm.Email />
						<LoginForm.Captcha />
						<MutedText>
							By clicking Continue, you agree to the Rivet{" "}
							<Link
								href="https://rivet.gg/terms"
								target="_blank"
								rel="noreferrer"
							>
								Terms of Service
							</Link>{" "}
							and{" "}
							<Link
								href="https://rivet.gg/support"
								target="_blank"
								rel="noreferrer"
							>
								Privacy Policy
							</Link>
							.
						</MutedText>
					</Flex>
				</CardContent>
				<CardFooter>
					<LoginForm.Submit w="full">Continue</LoginForm.Submit>
				</CardFooter>
			</AuthCard>
		</LoginForm.Form>
	);
};
