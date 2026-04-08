# Better-Auth Missing Flows — Design Spec

**Date:** 2026-04-08

## Overview

After migrating from Clerk to better-auth, four auth/org flows are missing. This spec covers all four as independent, parallelizable implementation tasks.

## Shared Patterns

All new UI follows existing conventions:
- Routes in `frontend/src/routes/`
- Page components in `frontend/src/app/`
- Dialogs as lazy-loaded frames in `frontend/src/app/dialogs/`, registered in `frontend/src/app/use-dialog.tsx`
- Forms in `frontend/src/components/forms/` using controlled `Form` wrappers with `RootError`
- `authClient` from `frontend/src/lib/auth.ts` (better-auth React client with `organizationClient` plugin)
- Cards styled with `Card`, `CardHeader`, `CardContent`, `CardFooter` from `@/components/ui/card`
- Auth routes guard with `features.auth` check and redirect to `/login` if unauthenticated

---

## Task 1: Email Verification Flow

### Scope
- Post-sign-up success state in `SignUp` component
- New route `/verify-email` for the email link landing page

### Sign-up success state
After `authClient.signUp.email()` succeeds, replace the form content inline with a "Check your inbox" message inside the same `Card`. Add a `verified` state boolean to `SignUp`. When `true`, swap `<Form>` children for a static message:

```
Title: "Check your email"
Body:  "We sent a verification link to <email>. Click it to activate your account."
Footer: "Resend email" (calls authClient.sendVerificationEmail) + "Back to sign in" link
```

OAuth sign-up (`handleGoogleSignUp`) skips this — redirect immediately as before.

### `/verify-email` route
File: `frontend/src/routes/verify-email.tsx`
Component file: `frontend/src/app/verify-email.tsx`

- Reads `?token=` from search params
- On mount calls `authClient.verifyEmail({ query: { token } })`
- Three UI states:
  - Loading: spinner
  - Success: "Email verified! Redirecting…" then `redirectToOrganization()`
  - Error: "This link is invalid or has expired." + "Request a new link" button (calls `authClient.sendVerificationEmail` if session exists, else links to `/join`)
- No auth required to load this route (link arrives in email before login)

---

## Task 2: Reset Password Flow

### Scope
- "Forgot password?" link on the login page
- New route `/forgot-password` — email input form
- New route `/reset-password` — new password form (token from URL)

### Login page change
Add a small "Forgot password?" link below the `<PasswordField />` in `login.tsx`, linking to `/forgot-password`.

### `/forgot-password` route
File: `frontend/src/routes/forgot-password.tsx`
Component: `frontend/src/app/forgot-password.tsx`
Form: `frontend/src/components/forms/forgot-password-form.tsx`

Card layout matching `/login`:
```
Title: "Reset your password"
Body:  EmailField + RootError
Footer: Submit "Send reset link" + "Back to sign in" link
```

On submit: `authClient.forgetPassword({ email, redirectTo: window.location.origin + "/reset-password" })`

After success, swap form content inline for:
```
"Reset link sent. Check your inbox."
```

### `/reset-password` route
File: `frontend/src/routes/reset-password.tsx`
Component: `frontend/src/app/reset-password.tsx`
Form: `frontend/src/components/forms/reset-password-form.tsx`

Reads `?token=` from search params. Card layout:
```
Title: "Choose a new password"
Body:  PasswordField (label "New password") + PasswordField (label "Confirm password") + RootError
Footer: Submit "Set new password"
```

On submit: validates passwords match client-side, calls `authClient.resetPassword({ newPassword, token })`.

On success: redirect to `/login` with a success toast/message.
On error (expired token): show error with "Request a new link" → `/forgot-password`.

---

## Task 3: Org Members Dialog

### Scope
- New dialog frame `org-members-frame.tsx`
- "Manage Members" entry in user dropdown
- Registered in `useDialog`

### Dialog frame
File: `frontend/src/app/dialogs/org-members-frame.tsx`

Three sections:

**Members list** — calls `authClient.organization.getMembers({ query: { organizationId } })`. Each row: avatar, name/email, role badge, "Remove" button (calls `authClient.organization.removeMember`). Current user's row has no Remove button.

**Invite member form** — inline form below the list:
```
EmailField + role select (owner | admin | member, default member) + "Send Invite" button
```
Calls `authClient.organization.inviteMember({ email, role, organizationId })`.

**Pending invitations** — calls `authClient.organization.listInvitations({ query: { organizationId } })`. Each row: email, role, "Revoke" button (calls `authClient.organization.cancelInvitation`). Hidden section if empty.

### User dropdown change
In `frontend/src/app/user-dropdown.tsx`, add a "Manage Members" `DropdownMenuItem` above "Logout" (only when `params.organization` exists). Opens dialog via `navigate` with `modal: "org-members"` search param, following the same pattern as other modals.

### `useDialog` registration
Add `OrgMembers: createDialogHook(() => import("@/app/dialogs/org-members-frame"))` to `frontend/src/app/use-dialog.tsx`.

Add `"org-members"` to the modal enum in `frontend/src/routes/_context.tsx` and render `<OrgMembersDialog>` in `CloudModals`.

---

## Task 4: Org Invitation Landing Page

### Scope
- New route `/accept-invitation` — landing page for invited users

### Route
File: `frontend/src/routes/accept-invitation.tsx`
Component: `frontend/src/app/accept-invitation.tsx`

Reads `?invitationId=` from search params (better-auth puts this in the link).

**Auth-aware rendering:**
- If user is not logged in: show a card with the org name (fetched from the invitation details if the API allows unauthenticated lookup, otherwise just a generic message), with "Sign in to accept" and "Create account to accept" buttons. After auth, the user returns to this URL (pass `callbackURL` to social sign-in; redirect `from` for email sign-in).
- If user is logged in: show "You've been invited to join [Org Name]" with "Accept" and "Decline" buttons.

On accept: `authClient.organization.acceptInvitation({ invitationId })` → redirect to org.
On decline: `authClient.organization.rejectInvitation({ invitationId })` → redirect to `/`.
On error (expired/invalid): show error message with link to contact org admin.

No `beforeLoad` auth guard — the page must render for unauthenticated users.

---

## Implementation Notes

- Each task is fully independent and can be assigned to a separate agent.
- All new routes should have `features.auth` guards where appropriate (Tasks 1, 2 public; Tasks 3, 4 see spec above).
- Use `authClient.useSession()` in React components for reactive session state.
- No new dependencies required — better-auth's `organizationClient` plugin already includes all needed methods.
- Invitation list/revoke methods: confirm exact API names against `better-auth@1.5.6` docs before implementing (`authClient.organization.listInvitations`, `authClient.organization.cancelInvitation`).
