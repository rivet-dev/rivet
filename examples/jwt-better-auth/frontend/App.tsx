import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { createClient, type ActorConn } from "rivetkit/client";
import type { user, registry } from "../src/actors.ts";
import { demoAccount } from "../demo-account.ts";
import { authClient, getActorToken, login, signUp } from "./login.ts";

type Credentials = { email: string; password: string };
type Session = { email: string; user: ActorConn<typeof user>; count: number };

export default function App() {
	const [isSignUp, setIsSignUp] = useState(false);
	const [session, setSession] = useState<Session | null>(null);
	const [updating, setUpdating] = useState(false);
	const [actionError, setActionError] = useState("");
	const {
		register,
		handleSubmit,
		reset,
		clearErrors,
		setError,
		formState: { errors, isSubmitting },
	} = useForm<Credentials>();
	const actorConnection = session?.user;

	useEffect(() => {
		return () => {
			void actorConnection?.dispose();
		};
	}, [actorConnection]);

	async function refreshToken() {
		try {
			return await getActorToken();
		} catch (error) {
			if (error instanceof Error && error.cause === 401) {
				setSession(null);
				setError("root", { message: error.message });
			}
			throw error;
		}
	}

	async function signIn(credentials: Credentials) {
		let connection: ActorConn<typeof user> | undefined;
		try {
			// Better Auth establishes a session. RivetKit uses it to request actor tokens.
			const { endpoint, namespace, actorId } = await (isSignUp
				? signUp(credentials)
				: login(credentials));
			const client = createClient<typeof registry>({
				endpoint,
				namespace,
				getToken: refreshToken,
			});
			connection = client.user.getForId(actorId).connect();
			setSession({
				email: credentials.email,
				user: connection,
				count: await connection.getCount(),
			});
			reset();
			setIsSignUp(false);
		} catch (error) {
			void connection?.dispose();
			setError("root", {
				message:
					error instanceof Error ? error.message : "Login failed",
			});
		}
	}

	async function increment() {
		if (!session) return;
		setUpdating(true);
		setActionError("");
		try {
			const count = await session.user.increment(1);
			setSession((current) => (current ? { ...current, count } : null));
		} catch {
			setActionError("Could not update the counter. Please try again.");
		} finally {
			setUpdating(false);
		}
	}

	async function signOut() {
		setUpdating(true);
		try {
			const { error } = await authClient.signOut();
			if (error) throw new Error(error.message);
			setSession(null);
			setActionError("");
		} catch {
			setActionError("Could not log out. Please try again.");
		} finally {
			setUpdating(false);
		}
	}

	return (
		<main>
			<p className="eyebrow">RivetKit example</p>
			<h1>
				{session
					? "Your user actor"
					: isSignUp
						? "Create an account"
						: "Log in"}
			</h1>
			<p className="intro">Every user gets their own counter.</p>
			{session ? (
				<section aria-label="Your user actor">
					<p className="status">Logged in as {session.email}</p>
					<p className="count" aria-live="polite" aria-label="Count">
						{session.count}
					</p>
					<button
						type="button"
						onClick={increment}
						disabled={updating}
					>
						{updating ? "Updating…" : "Increment"}
					</button>
					<button
						className="secondary"
						type="button"
						disabled={updating}
						onClick={signOut}
					>
						Log out
					</button>
					{actionError && <p role="alert">{actionError}</p>}
					<p className="hint">
						Actor access renews automatically while you’re logged
						in. Your count is saved.
					</p>
				</section>
			) : (
				<form onSubmit={handleSubmit(signIn)}>
					<label htmlFor="email">Email</label>
					<input
						id="email"
						type="email"
						autoComplete="username"
						required
						{...register("email")}
					/>
					<label htmlFor="password">Password</label>
					<input
						id="password"
						type="password"
						autoComplete="current-password"
						required
						{...register("password")}
					/>
					<button type="submit" disabled={isSubmitting}>
						{isSubmitting
							? "Please wait…"
							: isSignUp
								? "Create account"
								: "Log in"}
					</button>
					<button
						type="button"
						className="secondary"
						disabled={isSubmitting}
						onClick={() => {
							setIsSignUp(!isSignUp);
							clearErrors();
						}}
					>
						{isSignUp
							? "Already have an account? Log in"
							: "New here? Create an account"}
					</button>
					{errors.root && <p role="alert">{errors.root.message}</p>}
					{!isSignUp && (
						<p className="hint">
							<strong>Demo account</strong>
							<br />
							Email: <code>{demoAccount.email}</code>
							<br />
							Password: <code>{demoAccount.password}</code>
						</p>
					)}
				</form>
			)}
		</main>
	);
}
