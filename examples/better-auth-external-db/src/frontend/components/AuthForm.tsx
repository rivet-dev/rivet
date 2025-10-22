import { useState } from "react";
import { authClient } from "../auth-client";

interface AuthFormProps {
	onAuthSuccess: () => void;
}

export function AuthForm({ onAuthSuccess }: AuthFormProps) {
	const [isLogin, setIsLogin] = useState(true);
	const [email, setEmail] = useState("");
	const [name, setName] = useState("");
	const [password, setPassword] = useState("");
	const [error, setError] = useState("");
	const [loading, setLoading] = useState(false);

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		setError("");
		setLoading(true);

		try {
			if (isLogin) {
				await authClient.signIn.email({
					email,
					password,
				});
			} else {
				await authClient.signUp.email({
					email,
					name,
					password,
				});
			}
			onAuthSuccess();
		} catch (err) {
			setError(err instanceof Error ? err.message : "Authentication failed");
		} finally {
			setLoading(false);
		}
	};

	return (
		<div style={{ maxWidth: "400px", margin: "0 auto", padding: "20px" }}>
			<h2>{isLogin ? "Sign In" : "Sign Up"}</h2>

			<form
				onSubmit={handleSubmit}
				style={{ display: "flex", flexDirection: "column", gap: "15px" }}
			>
				<div>
					<label htmlFor="email">Email:</label>
					<input
						id="email"
						type="email"
						value={email}
						onChange={(e) => setEmail(e.target.value)}
						required
						style={{ width: "100%", padding: "8px", marginTop: "5px" }}
					/>
				</div>

				{!isLogin && (
					<div>
						<label htmlFor="name">Name:</label>
						<input
							id="name"
							type="text"
							value={name}
							onChange={(e) => setName(e.target.value)}
							required
							style={{ width: "100%", padding: "8px", marginTop: "5px" }}
						/>
					</div>
				)}

				<div>
					<label htmlFor="password">Password:</label>
					<input
						id="password"
						type="password"
						value={password}
						onChange={(e) => setPassword(e.target.value)}
						required
						style={{ width: "100%", padding: "8px", marginTop: "5px" }}
					/>
				</div>

				{error && <div style={{ color: "red", fontSize: "14px" }}>{error}</div>}

				<button
					type="submit"
					disabled={loading}
					style={{
						padding: "10px",
						backgroundColor: "#007bff",
						color: "white",
						border: "none",
						borderRadius: "4px",
						cursor: loading ? "not-allowed" : "pointer",
					}}
				>
					{loading ? "Loading..." : isLogin ? "Sign In" : "Sign Up"}
				</button>
			</form>

			<div style={{ textAlign: "center", marginTop: "15px" }}>
				<button
					type="button"
					onClick={() => setIsLogin(!isLogin)}
					style={{
						background: "none",
						border: "none",
						color: "#007bff",
						cursor: "pointer",
						textDecoration: "underline",
					}}
				>
					{isLogin
						? "Need an account? Sign up"
						: "Already have an account? Sign in"}
				</button>
			</div>
		</div>
	);
}
