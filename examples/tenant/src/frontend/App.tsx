import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { Member, registry } from "../backend/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

const ORG_ID = "org-1";

export function App() {
	// Authentication state
	const [token, setToken] = useState<string>("");
	
	// Data state
	const [organization, setOrganization] = useState<any>(null);
	const [members, setMembers] = useState<Member[]>([]);
	const [dashboardStats, setDashboardStats] = useState<any>(null);
	const [error, setError] = useState<string>("");
	const [loading, setLoading] = useState(false);

	// Connect to tenant actor with authentication token
	const tenant = useActor({
		name: "tenant",
		key: [ORG_ID],
		params: { token },
	});

	// Login functions
	const loginAsAdmin = () => {
		setToken("auth:user-1"); // Alice is admin
		setError("");
	};

	const loginAsMember = () => {
		setToken("auth:user-2"); // Bob is member
		setError("");
	};

	const loginAsCharlie = () => {
		setToken("auth:user-3"); // Charlie is member
		setError("");
	};

	const logout = () => {
		setToken("");
		setOrganization(null);
		setMembers([]);
		setDashboardStats(null);
		setError("");
	};

	// Load data when actor is available
	useEffect(() => {
		if (!tenant.connection || !token) return;

		const loadData = async () => {
			setLoading(true);
			try {
				// Get organization info
				const org = await tenant.connection!.getOrganization();
				setOrganization(org);

				// Get members (available to all users)
				const membersList = await tenant.connection!.getMembers();
				setMembers(membersList);

				// Get dashboard stats
				const stats = await tenant.connection!.getDashboardStats();
				setDashboardStats(stats);
			} catch (err: any) {
				setError(err.message || "Failed to load data");
			} finally {
				setLoading(false);
			}
		};

		loadData();
	}, [tenant.connection, token]);

	// Listen for real-time updates
	tenant.useEvent("memberAdded", ({ member }: { member: Member }) => {
		setMembers(prev => [...prev, member]);
	});

	tenant.useEvent("memberUpdated", ({ member }: { member: Member }) => {
		setMembers(prev => prev.map(m => m.id === member.id ? member : m));
	});



	// Login screen when not authenticated
	if (!token) {
		return (
			<div className="app-container">
				<div className="header">
					<h1>Organization Dashboard</h1>
					<p>Multi-tenant role-based access control with RivetKit</p>
				</div>

				<div className="info-box">
					<h3>How it works</h3>
					<p>
						This tenant system demonstrates role-based access control in a multi-tenant environment. 
						Different user roles have different permissions - admins can access invoices and manage members, 
						while regular members can only view member information.
					</p>
				</div>

				<div className="login-section">
					<h2>Choose a User to Login</h2>
					<p>Select a user to see different permission levels:</p>
					<div className="login-buttons">
						<button className="login-button admin" onClick={loginAsAdmin}>
							Login as Alice (Admin)
						</button>
						<button className="login-button member" onClick={loginAsMember}>
							Login as Bob (Member)
						</button>
						<button className="login-button member" onClick={loginAsCharlie}>
							Login as Charlie (Member)
						</button>
					</div>
				</div>
			</div>
		);
	}

	return (
		<div className="app-container">
			<div className="header">
				<h1>Organization Dashboard</h1>
				<p>Multi-tenant role-based access control with RivetKit</p>
			</div>

			{/* User Info */}
			<div className="user-info">
				<div className="user-details">
					<span>Logged in</span>
				</div>
				<button className="logout-button" onClick={logout}>
					Logout
				</button>
			</div>

			{/* Organization Header */}
			{organization && (
				<div className="organization-header">
					<h2>{organization.name}</h2>
					<p>Organization ID: {organization.id} • {organization.memberCount} members</p>
				</div>
			)}

			{/* Loading State */}
			{loading && <div>Loading...</div>}

			{/* Error Display */}
			{error && (
				<div className="error-message">
					<h4>Access Denied</h4>
					<p>{error}</p>
				</div>
			)}

			{/* Dashboard Stats */}
			{dashboardStats && (
				<div className="section">
					<h3>Dashboard Statistics</h3>
					<div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "20px" }}>
						<div style={{ padding: "20px", backgroundColor: "#f8f9fa", borderRadius: "6px", textAlign: "center" }}>
							<div style={{ fontSize: "24px", fontWeight: "bold", color: "#007bff" }}>
								{dashboardStats.totalMembers}
							</div>
							<div style={{ color: "#6c757d" }}>Total Members</div>
						</div>
						<div style={{ padding: "20px", backgroundColor: "#f8f9fa", borderRadius: "6px", textAlign: "center" }}>
							<div style={{ fontSize: "24px", fontWeight: "bold", color: "#dc3545" }}>
								{dashboardStats.adminCount}
							</div>
							<div style={{ color: "#6c757d" }}>Admins</div>
						</div>
						<div style={{ padding: "20px", backgroundColor: "#f8f9fa", borderRadius: "6px", textAlign: "center" }}>
							<div style={{ fontSize: "24px", fontWeight: "bold", color: "#28a745" }}>
								{dashboardStats.memberCount}
							</div>
							<div style={{ color: "#6c757d" }}>Members</div>
						</div>
					</div>
				</div>
			)}

			{/* Members Section - available to all users */}
			<div className="section">
				<div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "15px" }}>
					<h3>Team Members</h3>
				</div>

				{members.length === 0 ? (
					<div className="empty-state">No members found</div>
				) : (
					<table className="data-table">
						<thead>
							<tr>
								<th>Name</th>
								<th>Email</th>
								<th>Role</th>
							</tr>
						</thead>
						<tbody>
							{members.map((member) => (
								<tr key={member.id}>
									<td>{member.name}</td>
									<td>{member.email}</td>
									<td>
										<span className={`role-badge ${member.role}`}>
											{member.role}
										</span>
									</td>
								</tr>
							))}
						</tbody>
					</table>
				)}
			</div>

		</div>
	);
}
