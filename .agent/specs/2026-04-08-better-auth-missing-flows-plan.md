# Better-Auth Missing Flows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add four missing auth/org flows to the frontend after migrating from Clerk to better-auth: email verification, password reset, org members management, and org invitation acceptance.

**Architecture:** Four fully independent tasks — each adds new routes and/or UI components following existing patterns (TanStack Router file-based routing, `Card`-based auth pages, lazy-loaded dialog frames). All API calls go through the existing `authClient` from `frontend/src/lib/auth.ts`.

**Tech Stack:** React, TanStack Router, better-auth 1.5.6 (organizationClient plugin), react-hook-form + zod via `createSchemaForm`, Tailwind/shadcn UI components.

**Each task is fully independent and can be assigned to a separate agent.**

---

## Confirmed better-auth API method names (v1.5.6)

- `authClient.sendVerificationEmail({ email, callbackURL? })` — send/resend verification email
- `authClient.verifyEmail({ query: { token } })` — verify with token from link
- `authClient.requestPasswordReset({ email, redirectTo? })` — send reset email
- `authClient.resetPassword({ newPassword, token? })` — set new password (token in body)
- `authClient.organization.listMembers({ query: { organizationId } })` — list org members
- `authClient.organization.createInvitation({ email, role, organizationId })` — invite by email
- `authClient.organization.listInvitations({ query: { organizationId } })` — list pending invitations
- `authClient.organization.cancelInvitation({ invitationId })` — cancel a pending invitation
- `authClient.organization.removeMember({ memberIdOrEmail, organizationId })` — remove member
- `authClient.organization.acceptInvitation({ invitationId })` — accept invitation
- `authClient.organization.rejectInvitation({ invitationId })` — reject invitation
- `authClient.useActiveOrganization()` — reactive hook returning `{ data: { id, name, slug, members, invitations } }`

---

## File Map

### Task 1 — Email Verification
- Modify: `frontend/src/app/sign-up.tsx` — add inline "check your email" success state
- Create: `frontend/src/routes/verify-email.tsx` — route definition
- Create: `frontend/src/app/verify-email.tsx` — verification landing page component

### Task 2 — Reset Password
- Modify: `frontend/src/app/login.tsx` — add "Forgot password?" link
- Create: `frontend/src/components/forms/forgot-password-form.tsx` — email form
- Create: `frontend/src/routes/forgot-password.tsx` — route definition
- Create: `frontend/src/app/forgot-password.tsx` — forgot password page component
- Create: `frontend/src/components/forms/reset-password-form.tsx` — new password form
- Create: `frontend/src/routes/reset-password.tsx` — route definition
- Create: `frontend/src/app/reset-password.tsx` — reset password page component

### Task 3 — Org Members Dialog
- Create: `frontend/src/app/dialogs/org-members-frame.tsx` — dialog with member list + invite form
- Modify: `frontend/src/app/use-dialog.tsx` — register OrgMembers dialog hook
- Modify: `frontend/src/app/user-dropdown.tsx` — add "Manage Members" menu item
- Modify: `frontend/src/routes/_context.tsx` — add `"org-members"` to modal enum, render dialog in CloudModals

### Task 4 — Org Invitation Landing Page
- Create: `frontend/src/routes/accept-invitation.tsx` — route definition
- Create: `frontend/src/app/accept-invitation.tsx` — invitation acceptance page component

---

## Task 1: Email Verification Flow

**Files:**
- Modify: `frontend/src/app/sign-up.tsx`
- Create: `frontend/src/routes/verify-email.tsx`
- Create: `frontend/src/app/verify-email.tsx`

### Step 1 — Add "check your email" state to sign-up

Replace `frontend/src/app/sign-up.tsx` with the version below. The key change: after `signUp.email()` succeeds, set `sentEmail` state and render a success card instead of the form.

```tsx
"use client";
import { faGoogle, Icon } from "@rivet-gg/icons";
import {
	isRedirect,
	Link,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { motion } from "framer-motion";
import { useState } from "react";
import {
	EmailField,
	Form,
	NameField,
	PasswordField,
	RootError,
	Submit,
	type SubmitHandler,
} from "@/components/forms/sign-up-form";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { TurnstileWidget } from "@/components/ui/turnstile";
import { authClient, redirectToOrganization } from "@/lib/auth";
import { cloudEnv } from "@/lib/env";
import { features } from "@/lib/features";

export function SignUp() {
	const navigate = useNavigate();
	const from = useSearch({ strict: false, select: (s) => s?.from as string });
	const [turnstileToken, setTurnstileToken] = useState<string | null>(null);
	const turnstileSiteKey = cloudEnv().VITE_APP_TURNSTILE_SITE_KEY;
	const [sentEmail, setSentEmail] = useState<string | null>(null);

	const handleSubmit: SubmitHandler = async (
		{ name, email, password },
		form,
	) => {
		if (features.captcha && !turnstileToken) {
			form.setError("root", {
				message: "Captcha verification is still loading, please try again",
			});
			return;
		}

		const result = await authClient.signUp.email(
			{ email, password, name },
			features.captcha && turnstileToken
				? { headers: { "x-captcha-response": turnstileToken } }
				: undefined,
		);

		if (result.error) {
			form.setError("root", {
				message: result.error.message ?? "Sign up failed",
			});
			return;
		}

		setTurnstileToken(null);

		// If already has a session (e.g. email verification not required server-side), redirect.
		const [error] = await attemptAsync(
			async () => await redirectToOrganization(),
		);

		if (error && isRedirect(error)) {
			return navigate(error.options);
		}

		// Email verification required — show the check-your-inbox state.
		setSentEmail(email);
	};

	const handleGoogleSignUp = async () => {
		await authClient.signIn.social({
			provider: "google",
			callbackURL: from ?? "/",
		});
	};

	const handleResend = async () => {
		if (!sentEmail) return;
		await authClient.sendVerificationEmail({ email: sentEmail });
	};

	if (sentEmail) {
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
							We sent a verification link to{" "}
							<span className="font-medium text-foreground">
								{sentEmail}
							</span>
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
					<CardTitle>Welcome!</CardTitle>
					<CardDescription>
						Create your account to get started.
					</CardDescription>
				</CardHeader>
				<Form
					defaultValues={{ name: "", email: "", password: "" }}
					onSubmit={handleSubmit}
				>
					<CardContent className="grid gap-y-4">
						<div className="grid grid-cols-1 gap-x-4">
							<Button
								variant="outline"
								type="button"
								onClick={handleGoogleSignUp}
							>
								<Icon icon={faGoogle} className="mr-2 size-4" />
								Google
							</Button>
						</div>
						<p className="flex items-center gap-x-3 text-sm text-muted-foreground before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border">
							or
						</p>
						<NameField />
						<EmailField />
						<PasswordField />
						<RootError />
						{features.captcha && turnstileSiteKey && (
							<TurnstileWidget
								siteKey={turnstileSiteKey}
								onSuccess={setTurnstileToken}
								onExpire={() => setTurnstileToken(null)}
								onError={() => setTurnstileToken(null)}
								onTimeout={() => setTurnstileToken(null)}
							/>
						)}
					</CardContent>
					<CardFooter>
						<div className="grid w-full gap-y-4">
							<Submit allowPristine>Continue</Submit>
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
				</Form>
			</Card>
		</motion.div>
	);
}
```

- [ ] Apply the above file content to `frontend/src/app/sign-up.tsx`

- [ ] **Commit**
```bash
git commit -m "feat(frontend): add email verification pending state to sign-up form"
```

---

### Step 2 — Create the verify-email component

Create `frontend/src/app/verify-email.tsx`:

```tsx
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
```

- [ ] Create `frontend/src/app/verify-email.tsx` with the above content

---

### Step 3 — Create the verify-email route

Create `frontend/src/routes/verify-email.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { VerifyEmail } from "@/app/verify-email";

export const Route = createFileRoute("/verify-email")({
	component: VerifyEmail,
});
```

- [ ] Create `frontend/src/routes/verify-email.tsx` with the above content

- [ ] **Verify the app builds:** Run `cd frontend && pnpm tsc --noEmit` and confirm no type errors in the new files.

- [ ] **Commit**
```bash
git commit -m "feat(frontend): add email verification landing page"
```

---

## Task 2: Reset Password Flow

**Files:**
- Modify: `frontend/src/app/login.tsx`
- Create: `frontend/src/components/forms/forgot-password-form.tsx`
- Create: `frontend/src/routes/forgot-password.tsx`
- Create: `frontend/src/app/forgot-password.tsx`
- Create: `frontend/src/components/forms/reset-password-form.tsx`
- Create: `frontend/src/routes/reset-password.tsx`
- Create: `frontend/src/app/reset-password.tsx`

### Step 1 — Add "Forgot password?" link to login page

In `frontend/src/app/login.tsx`, add a link between `<PasswordField />` and `<RootError />`:

```tsx
// After <PasswordField /> and before <RootError />:
<div className="flex justify-end">
    <Link
        to="/forgot-password"
        className="text-xs text-muted-foreground underline-offset-4 hover:underline"
    >
        Forgot password?
    </Link>
</div>
```

The `Link` import is already present in the file. The full updated `<CardContent>` block:

```tsx
<CardContent className="grid gap-y-4">
    <div className="grid grid-cols-1 gap-x-4">
        <Button
            variant="outline"
            type="button"
            onClick={handleGoogleSignIn}
        >
            <Icon icon={faGoogle} className="mr-2 size-4" />
            Google
        </Button>
    </div>
    <p className="flex items-center gap-x-3 text-sm text-muted-foreground before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border">
        or
    </p>
    <EmailField />
    <PasswordField />
    <div className="flex justify-end">
        <Link
            to="/forgot-password"
            className="text-xs text-muted-foreground underline-offset-4 hover:underline"
        >
            Forgot password?
        </Link>
    </div>
    <RootError />
    {features.captcha && turnstileSiteKey && (
        <TurnstileWidget
            siteKey={turnstileSiteKey}
            onSuccess={setTurnstileToken}
            onExpire={() => setTurnstileToken(null)}
            onError={() => setTurnstileToken(null)}
            onTimeout={() => setTurnstileToken(null)}
        />
    )}
</CardContent>
```

- [ ] Apply the CardContent change to `frontend/src/app/login.tsx`

- [ ] **Commit**
```bash
git commit -m "feat(frontend): add forgot password link to login page"
```

---

### Step 2 — Create forgot-password-form

Create `frontend/src/components/forms/forgot-password-form.tsx`:

```tsx
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { createSchemaForm } from "@/components/lib/create-schema-form";
import { FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";

export const formSchema = z.object({
	email: z.string().email("Invalid email address"),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const EmailField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="email"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Email address</FormLabel>
					<FormControl>
						<Input
							type="email"
							placeholder="you@company.com"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const RootError = () => {
	const { formState } = useFormContext<FormValues>();
	if (!formState.errors.root) return null;
	return (
		<p className="text-sm text-destructive">
			{formState.errors.root.message}
		</p>
	);
};
```

- [ ] Create `frontend/src/components/forms/forgot-password-form.tsx` with the above content

---

### Step 3 — Create forgot-password component and route

Create `frontend/src/app/forgot-password.tsx`:

```tsx
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
							We sent a password reset link. Check your inbox and follow
							the instructions.
						</CardDescription>
					</CardHeader>
					<CardFooter>
						<Button variant="link" className="text-primary-foreground" size="sm" asChild>
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
				<Form
					defaultValues={{ email: "" }}
					onSubmit={handleSubmit}
				>
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
```

Create `frontend/src/routes/forgot-password.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { ForgotPassword } from "@/app/forgot-password";
import { Logo } from "@/app/logo";

export const Route = createFileRoute("/forgot-password")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<ForgotPassword />
			</div>
		</div>
	);
}
```

- [ ] Create `frontend/src/app/forgot-password.tsx` with the above content
- [ ] Create `frontend/src/routes/forgot-password.tsx` with the above content

- [ ] **Commit**
```bash
git commit -m "feat(frontend): add forgot password page"
```

---

### Step 4 — Create reset-password-form

Create `frontend/src/components/forms/reset-password-form.tsx`:

```tsx
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { createSchemaForm } from "@/components/lib/create-schema-form";
import { FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";

export const formSchema = z
	.object({
		newPassword: z.string().min(8, "Password must be at least 8 characters"),
		confirmPassword: z.string().min(1, "Please confirm your password"),
	})
	.refine((data) => data.newPassword === data.confirmPassword, {
		message: "Passwords do not match",
		path: ["confirmPassword"],
	});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const NewPasswordField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="newPassword"
			render={({ field }) => (
				<FormItem>
					<FormLabel>New password</FormLabel>
					<FormControl>
						<Input
							type="password"
							placeholder="At least 8 characters"
							autoComplete="new-password"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const ConfirmPasswordField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="confirmPassword"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Confirm password</FormLabel>
					<FormControl>
						<Input
							type="password"
							placeholder="Repeat your new password"
							autoComplete="new-password"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const RootError = () => {
	const { formState } = useFormContext<FormValues>();
	if (!formState.errors.root) return null;
	return (
		<p className="text-sm text-destructive">
			{formState.errors.root.message}
		</p>
	);
};
```

- [ ] Create `frontend/src/components/forms/reset-password-form.tsx` with the above content

---

### Step 5 — Create reset-password component and route

Create `frontend/src/app/reset-password.tsx`:

```tsx
import { isRedirect, Link, useNavigate, useSearch } from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { motion } from "framer-motion";
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
				message: "Missing reset token. Please use the link from your email.",
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
								<Link to="/forgot-password">Request a new link</Link>
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
```

Create `frontend/src/routes/reset-password.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { Logo } from "@/app/logo";
import { ResetPassword } from "@/app/reset-password";

export const Route = createFileRoute("/reset-password")({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4">
			<div className="flex flex-col items-center gap-6 w-full">
				<Logo className="h-10 mb-4" />
				<ResetPassword />
			</div>
		</div>
	);
}
```

- [ ] Create `frontend/src/app/reset-password.tsx` with the above content
- [ ] Create `frontend/src/routes/reset-password.tsx` with the above content

- [ ] **Verify the app builds:** Run `cd frontend && pnpm tsc --noEmit` and confirm no type errors in the new files.

- [ ] **Commit**
```bash
git commit -m "feat(frontend): add reset password flow"
```

---

## Task 3: Org Members Dialog

**Files:**
- Create: `frontend/src/app/dialogs/org-members-frame.tsx`
- Modify: `frontend/src/app/use-dialog.tsx`
- Modify: `frontend/src/app/user-dropdown.tsx`
- Modify: `frontend/src/routes/_context.tsx`

### Step 1 — Create org-members-frame

Create `frontend/src/app/dialogs/org-members-frame.tsx`:

```tsx
import { faTrash, Icon } from "@rivet-gg/icons";
import { useState } from "react";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	type DialogContentProps,
	Frame,
	Skeleton,
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { authClient } from "@/lib/auth";

interface OrgMembersFrameContentProps extends DialogContentProps {}

export default function OrgMembersFrameContent({
	onClose,
}: OrgMembersFrameContentProps) {
	const { data: org, isPending } = authClient.useActiveOrganization();
	const { data: session } = authClient.useSession();

	const [inviteEmail, setInviteEmail] = useState("");
	const [inviteRole, setInviteRole] = useState<"member" | "admin" | "owner">(
		"member",
	);
	const [inviteError, setInviteError] = useState<string | null>(null);
	const [invitePending, setInvitePending] = useState(false);

	const handleInvite = async () => {
		if (!org || !inviteEmail.trim()) return;
		setInviteError(null);
		setInvitePending(true);

		const result = await authClient.organization.createInvitation({
			email: inviteEmail.trim(),
			role: inviteRole,
			organizationId: org.id,
		});

		setInvitePending(false);

		if (result.error) {
			setInviteError(result.error.message ?? "Failed to send invitation");
			return;
		}

		setInviteEmail("");
	};

	const handleRemoveMember = async (memberIdOrEmail: string) => {
		if (!org) return;
		await authClient.organization.removeMember({
			memberIdOrEmail,
			organizationId: org.id,
		});
	};

	const handleCancelInvitation = async (invitationId: string) => {
		await authClient.organization.cancelInvitation({ invitationId });
	};

	return (
		<>
			<Frame.Header>
				<Frame.Title>Manage Members</Frame.Title>
				<Frame.Description>
					View members and invite people to your organization.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content className="space-y-6 max-h-[60vh] overflow-y-auto">
				{isPending ? (
					<div className="space-y-2">
						<Skeleton className="w-full h-10" />
						<Skeleton className="w-full h-10" />
						<Skeleton className="w-full h-10" />
					</div>
				) : (
					<>
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>Member</TableHead>
									<TableHead>Role</TableHead>
									<TableHead className="w-min" />
								</TableRow>
							</TableHeader>
							<TableBody>
								{org?.members.length === 0 ? (
									<TableRow>
										<TableCell
											colSpan={3}
											className="text-center py-8 text-muted-foreground"
										>
											No members yet.
										</TableCell>
									</TableRow>
								) : (
									org?.members.map((member) => (
										<TableRow key={member.id}>
											<TableCell>
												<div className="flex items-center gap-2">
													<Avatar className="size-6">
														<AvatarImage
															src={
																(
																	member as {
																		user?: {
																			image?: string | null;
																		};
																	}
																).user?.image ??
																undefined
															}
														/>
														<AvatarFallback>
															{(
																(
																	member as {
																		user?: {
																			name?: string;
																			email?: string;
																		};
																	}
																).user?.name ??
																(
																	member as {
																		user?: {
																			email?: string;
																		};
																	}
																).user?.email ??
																"?"
															)[0].toUpperCase()}
														</AvatarFallback>
													</Avatar>
													<div className="text-sm">
														<p className="font-medium">
															{
																(
																	member as {
																		user?: {
																			name?: string;
																		};
																	}
																).user?.name
															}
														</p>
														<p className="text-muted-foreground">
															{
																(
																	member as {
																		user?: {
																			email?: string;
																		};
																	}
																).user?.email
															}
														</p>
													</div>
												</div>
											</TableCell>
											<TableCell>
												<Badge variant="secondary">
													{member.role}
												</Badge>
											</TableCell>
											<TableCell>
												{member.userId !==
													session?.user.id && (
													<Button
														variant="ghost"
														size="icon"
														onClick={() =>
															handleRemoveMember(
																member.userId,
															)
														}
													>
														<Icon
															icon={faTrash}
															className="size-4 text-destructive"
														/>
													</Button>
												)}
											</TableCell>
										</TableRow>
									))
								)}
							</TableBody>
						</Table>

						{(org?.invitations?.length ?? 0) > 0 && (
							<div className="space-y-2">
								<p className="text-sm font-medium text-muted-foreground">
									Pending invitations
								</p>
								<Table>
									<TableHeader>
										<TableRow>
											<TableHead>Email</TableHead>
											<TableHead>Role</TableHead>
											<TableHead className="w-min" />
										</TableRow>
									</TableHeader>
									<TableBody>
										{org?.invitations.map((inv) => (
											<TableRow key={inv.id}>
												<TableCell className="text-sm">
													{inv.email}
												</TableCell>
												<TableCell>
													<Badge variant="outline">
														{inv.role}
													</Badge>
												</TableCell>
												<TableCell>
													<Button
														variant="ghost"
														size="sm"
														onClick={() =>
															handleCancelInvitation(
																inv.id,
															)
														}
													>
														Revoke
													</Button>
												</TableCell>
											</TableRow>
										))}
									</TableBody>
								</Table>
							</div>
						)}

						<div className="space-y-3 pt-2 border-t">
							<p className="text-sm font-medium">Invite a member</p>
							<div className="flex gap-2">
								<div className="flex-1">
									<Label htmlFor="invite-email" className="sr-only">
										Email address
									</Label>
									<Input
										id="invite-email"
										type="email"
										placeholder="colleague@company.com"
										value={inviteEmail}
										onChange={(e) =>
											setInviteEmail(e.target.value)
										}
									/>
								</div>
								<Select
									value={inviteRole}
									onValueChange={(v) =>
										setInviteRole(
											v as "member" | "admin" | "owner",
										)
									}
								>
									<SelectTrigger className="w-28">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value="member">Member</SelectItem>
										<SelectItem value="admin">Admin</SelectItem>
										<SelectItem value="owner">Owner</SelectItem>
									</SelectContent>
								</Select>
								<Button
									onClick={handleInvite}
									isLoading={invitePending}
									disabled={!inviteEmail.trim()}
								>
									Invite
								</Button>
							</div>
							{inviteError && (
								<p className="text-sm text-destructive">
									{inviteError}
								</p>
							)}
						</div>
					</>
				)}
			</Frame.Content>
			<Frame.Footer>
				<Button variant="secondary" onClick={onClose}>
					Close
				</Button>
			</Frame.Footer>
		</>
	);
}
```

- [ ] Create `frontend/src/app/dialogs/org-members-frame.tsx` with the above content

---

### Step 2 — Register OrgMembers in useDialog

In `frontend/src/app/use-dialog.tsx`, add the `OrgMembers` entry to the exported `useDialog` object:

```tsx
// Add this import alongside the other dialog imports:
OrgMembers: createDialogHook(
    () => import("@/app/dialogs/org-members-frame"),
),
```

The full updated file:

```tsx
import { useDialog as baseUseDialog, createDialogHook } from "@/components";

export const useDialog = {
	...baseUseDialog,
	CreateNamespace: createDialogHook(
		() => import("@/app/dialogs/create-namespace-frame"),
	),
	CreateProject: createDialogHook(
		() => import("@/app/dialogs/create-project-frame"),
	),
	ConnectRivet: createDialogHook(
		() => import("@/app/dialogs/connect-rivet-frame"),
	),
	ConnectVercel: createDialogHook(
		() => import("@/app/dialogs/connect-vercel-frame"),
	),
	ConnectQuickVercel: createDialogHook(
		() => import("@/app/dialogs/connect-quick-vercel-frame"),
	),
	ConnectRailway: createDialogHook(
		() => import("@/app/dialogs/connect-railway-frame"),
	),
	ConnectQuickRailway: createDialogHook(
		() => import("@/app/dialogs/connect-quick-railway-frame"),
	),
	ConnectManual: createDialogHook(
		() => import("@/app/dialogs/connect-manual-frame"),
	),
	ConnectCloudflare: createDialogHook(
		() => import("@/app/dialogs/connect-cloudflare-frame"),
	),
	ConnectAws: createDialogHook(
		() => import("@/app/dialogs/connect-aws-frame"),
	),
	ConnectGcp: createDialogHook(
		() => import("@/app/dialogs/connect-gcp-frame"),
	),
	ConnectHetzner: createDialogHook(
		() => import("@/app/dialogs/connect-hetzner-frame"),
	),
	EditProviderConfig: createDialogHook(
		() => import("@/app/dialogs/edit-runner-config"),
	),
	DeleteConfig: createDialogHook(
		() => import("@/app/dialogs/confirm-delete-config-frame"),
	),
	DeleteNamespace: createDialogHook(
		() => import("@/app/dialogs/confirm-delete-namespace-frame"),
	),
	DeleteProject: createDialogHook(
		() => import("@/app/dialogs/confirm-delete-project-frame"),
	),
	Billing: createDialogHook(() => import("@/app/dialogs/billing-frame")),
	ProvideEngineCredentials: createDialogHook(
		() => import("@/app/dialogs/provide-engine-credentials-frame"),
	),
	Tokens: createDialogHook(() => import("@/app/dialogs/tokens-frame")),
	ApiTokens: createDialogHook(() => import("@/app/dialogs/api-tokens-frame")),
	CreateApiToken: createDialogHook(
		() => import("@/app/dialogs/create-api-token-frame"),
	),
	CreateOrganization: createDialogHook(
		() => import("@/app/dialogs/create-organization-frame"),
	),
	UpsertDeployment: createDialogHook(
		() => import("@/app/dialogs/upsert-deployment-frame"),
	),
	OrgMembers: createDialogHook(
		() => import("@/app/dialogs/org-members-frame"),
	),
};
```

- [ ] Apply the above to `frontend/src/app/use-dialog.tsx`

---

### Step 3 — Add modal enum value and CloudModals render in _context.tsx

In `frontend/src/routes/_context.tsx`:

1. Add `"org-members"` to the `modal` enum in `searchSchema`:

```tsx
modal: z
    .enum([
        "go-to-actor",
        "feedback",
        "create-ns",
        "create-project",
        "billing",
        "org-members",
    ])
    .or(z.string())
    .optional(),
```

2. Update `CloudModals` to render the OrgMembers dialog:

```tsx
function CloudModals() {
	const navigate = useNavigate();
	const search = useSearch({ strict: false });

	const CreateProjectDialog = useDialog.CreateProject.Dialog;
	const CreateOrganizationDialog = useDialog.CreateOrganization.Dialog;
	const OrgMembersDialog = useDialog.OrgMembers.Dialog;

	return (
		<>
			<CreateProjectDialog
				organization={search?.organization}
				dialogProps={{
					open: search?.modal === "create-project",
					onOpenChange: (value) => {
						if (!value) {
							return navigate({
								to: ".",
								search: (old) => ({ ...old, modal: undefined }),
							});
						}
					},
				}}
			/>
			<CreateOrganizationDialog
				dialogProps={{
					open: search?.modal === "create-organization",
					onOpenChange: (value) => {
						if (!value) {
							return navigate({
								to: ".",
								search: (old) => ({ ...old, modal: undefined }),
							});
						}
					},
				}}
			/>
			<OrgMembersDialog
				dialogProps={{
					open: search?.modal === "org-members",
					onOpenChange: (value) => {
						if (!value) {
							return navigate({
								to: ".",
								search: (old) => ({ ...old, modal: undefined }),
							});
						}
					},
				}}
			/>
		</>
	);
}
```

- [ ] Apply both changes to `frontend/src/routes/_context.tsx`

---

### Step 4 — Add "Manage Members" to user dropdown

In `frontend/src/app/user-dropdown.tsx`, add a "Manage Members" `DropdownMenuItem` inside the block that's already gated on `params.organization`, before the Logout item:

```tsx
{params.organization ? (
    <>
        <DropdownMenuSub>
            <DropdownMenuSubTrigger>
                Switch Organization
            </DropdownMenuSubTrigger>
            <DropdownMenuPortal>
                <DropdownMenuSubContent>
                    <OrganizationSwitcher
                        value={params.organization}
                    />
                </DropdownMenuSubContent>
            </DropdownMenuPortal>
        </DropdownMenuSub>
        <DropdownMenuItem
            onSelect={() => {
                navigate({
                    to: ".",
                    search: (old) => ({
                        ...old,
                        modal: "org-members",
                    }),
                });
            }}
        >
            Manage Members
        </DropdownMenuItem>
    </>
) : null}
```

- [ ] Apply the change to `frontend/src/app/user-dropdown.tsx`

- [ ] **Verify the app builds:** Run `cd frontend && pnpm tsc --noEmit` and confirm no type errors.

- [ ] **Commit**
```bash
git commit -m "feat(frontend): add org members management dialog"
```

---

## Task 4: Org Invitation Landing Page

**Files:**
- Create: `frontend/src/routes/accept-invitation.tsx`
- Create: `frontend/src/app/accept-invitation.tsx`

### Step 1 — Create accept-invitation component

Create `frontend/src/app/accept-invitation.tsx`:

```tsx
import { faGoogle, Icon } from "@rivet-gg/icons";
import { isRedirect, useNavigate, useSearch } from "@tanstack/react-router";
import { attemptAsync } from "es-toolkit";
import { useEffect, useState } from "react";
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

type Status = "loading" | "ready" | "accepting" | "error";

export function AcceptInvitation() {
	const navigate = useNavigate();
	const invitationId = useSearch({
		strict: false,
		select: (s) => s?.invitationId as string | undefined,
	});
	const token = useSearch({
		strict: false,
		select: (s) => s?.token as string | undefined,
	});
	const { data: session, isPending: sessionPending } =
		authClient.useSession();
	const [status, setStatus] = useState<Status>("loading");
	const [errorMessage, setErrorMessage] = useState<string | null>(null);
	const [orgName, setOrgName] = useState<string | null>(null);

	// Resolve the invitation ID — it may come as ?invitationId= or ?token=
	const resolvedInvitationId = invitationId ?? token;

	useEffect(() => {
		if (sessionPending) return;
		if (!resolvedInvitationId) {
			setStatus("error");
			setErrorMessage("No invitation ID found in this link.");
			return;
		}
		setStatus("ready");
	}, [sessionPending, resolvedInvitationId]);

	const handleAccept = async () => {
		if (!resolvedInvitationId) return;
		setStatus("accepting");

		const result = await authClient.organization.acceptInvitation({
			invitationId: resolvedInvitationId,
		});

		if (result.error) {
			setStatus("error");
			setErrorMessage(
				result.error.message ?? "Failed to accept invitation",
			);
			return;
		}

		const [error] = await attemptAsync(
			async () => await redirectToOrganization(),
		);

		if (error && isRedirect(error)) {
			navigate(error.options);
		}
	};

	const handleReject = async () => {
		if (!resolvedInvitationId) return;
		await authClient.organization.rejectInvitation({
			invitationId: resolvedInvitationId,
		});
		navigate({ to: "/" });
	};

	const handleGoogleSignIn = async () => {
		const callbackURL = window.location.href;
		await authClient.signIn.social({ provider: "google", callbackURL });
	};

	if (sessionPending || status === "loading") {
		return (
			<div className="flex min-h-screen items-center justify-center bg-background">
				<p className="text-muted-foreground text-sm">Loading…</p>
			</div>
		);
	}

	if (status === "error") {
		return (
			<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
				<Card className="w-full max-w-md">
					<CardHeader>
						<CardTitle>Invitation unavailable</CardTitle>
						<CardDescription>
							{errorMessage ??
								"This invitation link is invalid, expired, or has already been used."}
						</CardDescription>
					</CardHeader>
					<CardFooter>
						<Button variant="outline" onClick={() => navigate({ to: "/" })}>
							Go to dashboard
						</Button>
					</CardFooter>
				</Card>
			</div>
		);
	}

	if (!session) {
		return (
			<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
				<Card className="w-full max-w-md">
					<CardHeader>
						<CardTitle>You've been invited{orgName ? ` to ${orgName}` : ""}</CardTitle>
						<CardDescription>
							Sign in or create an account to accept this invitation.
						</CardDescription>
					</CardHeader>
					<CardContent className="grid gap-y-3">
						<Button onClick={handleGoogleSignIn}>
							<Icon icon={faGoogle} className="mr-2 size-4" />
							Continue with Google
						</Button>
						<Button
							variant="outline"
							onClick={() =>
								navigate({
									to: "/login",
									search: {
										from: window.location.pathname + window.location.search,
									},
								})
							}
						>
							Sign in with email
						</Button>
						<Button
							variant="ghost"
							onClick={() =>
								navigate({
									to: "/join",
									search: {
										from: window.location.pathname + window.location.search,
									},
								})
							}
						>
							Create a new account
						</Button>
					</CardContent>
				</Card>
			</div>
		);
	}

	return (
		<div className="flex min-h-screen flex-col items-center justify-center bg-background py-4 px-4">
			<Card className="w-full max-w-md">
				<CardHeader>
					<CardTitle>
						You've been invited{orgName ? ` to ${orgName}` : ""}
					</CardTitle>
					<CardDescription>
						Accept the invitation to join the organization.
					</CardDescription>
				</CardHeader>
				<CardFooter className="gap-2">
					<Button
						onClick={handleAccept}
						isLoading={status === "accepting"}
					>
						Accept invitation
					</Button>
					<Button variant="outline" onClick={handleReject}>
						Decline
					</Button>
				</CardFooter>
			</Card>
		</div>
	);
}
```

- [ ] Create `frontend/src/app/accept-invitation.tsx` with the above content

---

### Step 2 — Create accept-invitation route

Create `frontend/src/routes/accept-invitation.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { AcceptInvitation } from "@/app/accept-invitation";
import { features } from "@/lib/features";

export const Route = createFileRoute("/accept-invitation")({
	component: features.auth ? AcceptInvitation : () => null,
});
```

- [ ] Create `frontend/src/routes/accept-invitation.tsx` with the above content

- [ ] **Verify the app builds:** Run `cd frontend && pnpm tsc --noEmit` and confirm no type errors.

- [ ] **Commit**
```bash
git commit -m "feat(frontend): add org invitation acceptance landing page"
```

---

## Self-Review Notes

- All four tasks are independent and share no state.
- `authClient.requestPasswordReset` (not `forgetPassword`) is the correct method name confirmed from `better-auth@1.5.6` types.
- `authClient.organization.createInvitation` (not `inviteMember`) is confirmed from the org plugin client.
- `authClient.useActiveOrganization()` in Task 3 returns members and invitations as part of the active org — no extra API calls needed.
- The `invitationId` URL param name matches what better-auth puts in invitation email links by default (verify this against the server configuration if links use a different param name like `token`).
- Task 3's member row user data (`member.user.name`, `member.user.email`) is accessed via type assertions because better-auth's `InferMember` type doesn't always include the joined user object in its TS type — the actual runtime response includes it from `getFullOrganization`. If TypeScript errors arise here, add a local type cast or extract `user` from the member response explicitly.
