import { useEffect, useState } from "react";
import { authClient } from "./auth-client";
import { AuthForm } from "./components/AuthForm";
import { ChatRoom } from "./components/ChatRoom";

function App() {
	const [user, setUser] = useState<{ id: string; email: string } | null>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		// Check if user is already authenticated
		const checkAuth = async () => {
			try {
				const session = await authClient.getSession();
				if (session.data?.user) {
					setUser(session.data.user);
				}
			} catch (error) {
				console.error("Auth check failed:", error);
			} finally {
				setLoading(false);
			}
		};

		checkAuth();
	}, []);

	const handleAuthSuccess = async () => {
		try {
			const session = await authClient.getSession();
			if (session.data?.user) {
				setUser(session.data.user);
			}
		} catch (error) {
			console.error("Failed to get user after auth:", error);
		}
	};

	const handleSignOut = () => {
		setUser(null);
	};

	if (loading) {
		return (
			<div
				style={{
					display: "flex",
					justifyContent: "center",
					alignItems: "center",
					height: "100vh",
				}}
			>
				Loading...
			</div>
		);
	}

	return (
		<div style={{ minHeight: "100vh", backgroundColor: "#f0f0f0" }}>
			<div style={{ padding: "20px 0" }}>
				<h1 style={{ textAlign: "center", marginBottom: "30px" }}>
					RivetKit with Better Auth
				</h1>

				{user ? (
					<ChatRoom user={user} onSignOut={handleSignOut} />
				) : (
					<AuthForm onAuthSuccess={handleAuthSuccess} />
				)}
			</div>
		</div>
	);
}

export default App;
