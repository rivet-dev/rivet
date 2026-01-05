import { ActorError, createClient } from "rivetkit/client";
import { useState } from "react";
import type {
	CompanyProfile,
	EmployeeProfile,
	registry,
} from "../src/registry";

const client = createClient<typeof registry>(`${window.location.origin}/api/rivet`);

export function App() {
	const [companyEin, setCompanyEin] = useState("12-3456789");
	const [companyName, setCompanyName] = useState("Acme Corp");
	const [companyIndustry, setCompanyIndustry] = useState("Technology");
	const [companyProfile, setCompanyProfile] = useState<CompanyProfile | null>(
		null
	);

	const [employeeName, setEmployeeName] = useState("Jane Smith");
	const [employeeEmail, setEmployeeEmail] = useState("jane@acme.com");
	const [employeePosition, setEmployeePosition] = useState("Software Engineer");
	const [createdEmployee, setCreatedEmployee] =
		useState<EmployeeProfile | null>(null);
	const [employeeList, setEmployeeList] = useState<string[]>([]);

	const createCompany = async () => {
		try {
			// Create actor with input using EIN as key
			const company = await client.company.create([companyEin], {
				input: {
					name: companyName,
					industry: companyIndustry,
				},
			});

			// Get profile with full type safety
			const profile = await company.getProfile();
			setCompanyProfile(profile);
		} catch (error) {
			// Handle actor already exists error
			if (
				error instanceof ActorError &&
				error.group === "actor" &&
				error.code === "already_exists"
			) {
				alert(
					`Company with EIN "${companyEin}" already exists. Use "Load Profile" to view it.`
				);
			} else {
				// Re-throw other errors
				throw error;
			}
		}
	};

	const loadCompanyProfile = async () => {
		const company = client.company.get([companyEin]);
		const profile = await company.getProfile();
		setCompanyProfile(profile);

		// Also load the list of employees
		const employees = await company.getEmployees();
		setEmployeeList(employees);
	};

	const createEmployee = async () => {
		try {
			// Use the company's createEmployee action to spawn an employee actor
			const company = client.company.get([companyEin]);
			const employee = await company.createEmployee({
				name: employeeName,
				email: employeeEmail,
				position: employeePosition,
			});
			setCreatedEmployee(employee);

			// Refresh the employee list
			const employees = await company.getEmployees();
			setEmployeeList(employees);
		} catch (error) {
			// Handle actor already exists error
			if (
				error instanceof ActorError &&
				error.group === "actor" &&
				error.code === "already_exists"
			) {
				alert(
					`Employee with email "${employeeEmail}" already exists. Click on their name in the list to view their profile.`
				);
			} else {
				// Re-throw other errors
				throw error;
			}
		}
	};

	const loadEmployee = async (email: string) => {
		const employee = client.employee.get([email]);
		const profile = await employee.getProfile();
		setCreatedEmployee(profile);
	};


	return (
		<div className="container">
			<div className="header">
				<h1>Quickstart: Actions</h1>
				<p>
					Demonstrates creating actors with input parameters and calling
					type-safe actions between actors
				</p>
			</div>

			<div className="grid">
				{/* Company Section */}
				<div className="section">
					<h2>Company Actor</h2>
					<div className="form-group">
						<label>EIN (key):</label>
						<input
							type="text"
							value={companyEin}
							onChange={(e) => setCompanyEin(e.target.value)}
							placeholder="Enter EIN"
						/>
					</div>
					<div className="form-group">
						<label>Company Name:</label>
						<input
							type="text"
							value={companyName}
							onChange={(e) => setCompanyName(e.target.value)}
							placeholder="Enter company name"
						/>
					</div>
					<div className="form-group">
						<label>Industry:</label>
						<input
							type="text"
							value={companyIndustry}
							onChange={(e) => setCompanyIndustry(e.target.value)}
							placeholder="Enter industry"
						/>
					</div>
					<div className="button-group">
						<button
							onClick={createCompany}
							className="primary"
						>
							Create Company
						</button>
						<button
							onClick={loadCompanyProfile}
						>
							Load Profile
						</button>
					</div>

					{companyProfile && (
						<div className="profile">
							<h3>Company Profile</h3>
							<div className="profile-field">
								<strong>ID:</strong> {companyProfile.id}
							</div>
							<div className="profile-field">
								<strong>Name:</strong> {companyProfile.name}
							</div>
							<div className="profile-field">
								<strong>Industry:</strong> {companyProfile.industry}
							</div>
							<div className="profile-field">
								<strong>Founded:</strong>{" "}
								{new Date(companyProfile.foundedAt).toLocaleString()}
							</div>
						</div>
					)}

					{employeeList.length > 0 && (
						<div className="profile">
							<h3>Employees ({employeeList.length})</h3>
							<div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
								{employeeList.map((email) => (
									<button
										key={email}
										onClick={() => loadEmployee(email)}
										style={{
											padding: "8px 12px",
											textAlign: "left",
											cursor: "pointer",
										}}
									>
										{email}
									</button>
								))}
							</div>
						</div>
					)}
				</div>

				{/* Employee Section */}
				<div className="section">
					<h2>Employee Actor</h2>
					<p className="section-description">
						Create employees through the company actor
					</p>
					<div className="form-group">
						<label>Employee Email (key):</label>
						<input
							type="email"
							value={employeeEmail}
							onChange={(e) => setEmployeeEmail(e.target.value)}
							placeholder="Enter email"
						/>
					</div>
					<div className="form-group">
						<label>Name:</label>
						<input
							type="text"
							value={employeeName}
							onChange={(e) => setEmployeeName(e.target.value)}
							placeholder="Enter name"
						/>
					</div>
					<div className="form-group">
						<label>Position:</label>
						<input
							type="text"
							value={employeePosition}
							onChange={(e) => setEmployeePosition(e.target.value)}
							placeholder="Enter position"
						/>
					</div>
					<div className="button-group">
						<button onClick={createEmployee} className="primary">
							Create Employee via Company
						</button>
					</div>

					{createdEmployee && (
						<div className="profile">
							<h3>Employee Profile</h3>
							<div className="profile-field">
								<strong>Employee ID:</strong> {createdEmployee.employeeId}
							</div>
							<div className="profile-field">
								<strong>Name:</strong> {createdEmployee.name}
							</div>
							<div className="profile-field">
								<strong>Email:</strong> {createdEmployee.email}
							</div>
							<div className="profile-field">
								<strong>Position:</strong> {createdEmployee.position}
							</div>
							<div className="profile-field">
								<strong>Company ID:</strong> {createdEmployee.companyId}
							</div>
							<div className="profile-field">
								<strong>Hired:</strong>{" "}
								{new Date(createdEmployee.hiredAt).toLocaleString()}
							</div>
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
