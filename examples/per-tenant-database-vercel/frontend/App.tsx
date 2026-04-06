import { createRivetKit } from "@rivetkit/react";
import { type FormEvent, useEffect, useMemo, useState } from "react";
import type {
	CompanyStats,
	Employee,
	Project,
	ProjectStatus,
	registry,
} from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`,
);

const COMPANY_PRESETS = [
	"Aurora Analytics",
	"Cedarline Logistics",
	"Juniper Devices",
	"Mariner Studio",
];

const STATUS_LABELS: Record<ProjectStatus, string> = {
	planning: "Planning",
	active: "Active",
	done: "Done",
};

const formatDate = (timestamp: number) =>
	new Date(timestamp).toLocaleString();

const getInitials = (name: string) =>
	name
		.split(" ")
		.filter(Boolean)
		.map((part) => part[0])
		.join("")
		.slice(0, 2)
		.toUpperCase();

export function App() {
	const [companyName, setCompanyName] = useState<string | null>(null);
	const [companyInput, setCompanyInput] = useState(COMPANY_PRESETS[0]);
	const [preset, setPreset] = useState(COMPANY_PRESETS[0]);

	const handlePresetChange = (value: string) => {
		setPreset(value);
		setCompanyInput(value);
	};

	const handleSignIn = () => {
		const trimmed = companyInput.trim();
		if (trimmed.length === 0) return;
		setCompanyName(trimmed);
	};

	return (
		<div className="dashboard">
			<section className="hero">
				<div className="hero-badge">Per-tenant database demo</div>
				<h1>Isolate every company with a single Rivet Actor</h1>
				<p>
					Each company gets its own CompanyDatabase actor keyed by name. The
					actor state is the database, so switching companies swaps the
					entire dataset.
				</p>
			</section>

			{companyName ? (
				<CompanyDashboard
					companyName={companyName}
					onSwitch={setCompanyName}
				/>
			) : (
				<section className="card signin-card">
					<h2 className="signin-title">Sign in to a company</h2>
					<p className="signin-description">
						Pick a company or enter a new one. The company name becomes the
						actor key.
					</p>
					<div className="switcher">
						<label htmlFor="company-preset">Quick pick</label>
						<select
							id="company-preset"
							value={preset}
							onChange={(event) => handlePresetChange(event.target.value)}
						>
							{COMPANY_PRESETS.map((company) => (
								<option key={company} value={company}>
									{company}
								</option>
							))}
						</select>

						<label htmlFor="company-input">Company name</label>
						<input
							id="company-input"
							value={companyInput}
							onChange={(event) => setCompanyInput(event.target.value)}
							placeholder="Enter company name"
						/>

						<button type="button" onClick={handleSignIn}>
							Continue to dashboard
						</button>
					</div>
				</section>
			)}
		</div>
	);
}

type CompanyDashboardProps = {
	companyName: string;
	onSwitch: (companyName: string | null) => void;
};

function CompanyDashboard({ companyName, onSwitch }: CompanyDashboardProps) {
	const company = useActor({
		name: "companyDatabase",
		key: [companyName],
	});

	const [employees, setEmployees] = useState<Employee[]>([]);
	const [projects, setProjects] = useState<Project[]>([]);
	const [stats, setStats] = useState<CompanyStats | null>(null);
	const [employeeName, setEmployeeName] = useState("");
	const [employeeRole, setEmployeeRole] = useState("");
	const [projectName, setProjectName] = useState("");
	const [projectStatus, setProjectStatus] = useState<ProjectStatus>(
		"planning",
	);
	const [switchName, setSwitchName] = useState(companyName);

	useEffect(() => {
		setSwitchName(companyName);
	}, [companyName]);

	const refreshStats = async () => {
		if (!company.connection) return;
		const latestStats = await company.connection.getStats();
		setStats(latestStats);
	};

	useEffect(() => {
		let canceled = false;
		setEmployees([]);
		setProjects([]);
		setStats(null);

		const loadData = async () => {
			if (!company.connection) return;
			const [employeeRows, projectRows, latestStats] = await Promise.all([
				company.connection.listEmployees(),
				company.connection.listProjects(),
				company.connection.getStats(),
			]);

			if (canceled) return;
			setEmployees(employeeRows);
			setProjects(projectRows);
			setStats(latestStats);
		};

		loadData();

		return () => {
			canceled = true;
		};
	}, [company.connection, companyName]);

	company.useEvent("employeeAdded", (employee: Employee) => {
		setEmployees((prev) => [...prev, employee]);
		void refreshStats();
	});

	company.useEvent("projectAdded", (project: Project) => {
		setProjects((prev) => [...prev, project]);
		void refreshStats();
	});

	const companyInitials = useMemo(() => getInitials(companyName), [companyName]);

	const handleAddEmployee = async (event: FormEvent) => {
		event.preventDefault();
		if (!company.connection) return;
		await company.connection.addEmployee(employeeName, employeeRole);
		setEmployeeName("");
		setEmployeeRole("");
	};

	const handleAddProject = async (event: FormEvent) => {
		event.preventDefault();
		if (!company.connection) return;
		await company.connection.addProject(projectName, projectStatus);
		setProjectName("");
	};

	const handleSwitchCompany = () => {
		const trimmed = switchName.trim();
		if (!trimmed) return;
		onSwitch(trimmed);
	};

	const connectionLabel = company.connection
		? "Connected to CompanyDatabase actor"
		: "Connecting to CompanyDatabase actor";

	return (
		<section className="card">
			<div className="company-banner">
				<div className="company-id">
					<div className="company-badge">{companyInitials}</div>
					<div>
						<h2 className="company-name">{companyName}</h2>
						<p className="company-subtitle">
							Each company name maps to a unique actor with isolated state.
						</p>
					</div>
				</div>
				<div className="company-meta">
					<span className="pill">Actor key: {companyName}</span>
					<span className="pill">{connectionLabel}</span>
				</div>
			</div>

			<div className="data-grid">
				<div className="panel">
					<div className="panel-header">
						<h3>Company stats</h3>
						<span className="count-chip">Live snapshot</span>
					</div>
					{stats ? (
						<div className="stats-grid">
							<div className="stat-card">
								<div className="stat-value">{stats.employee_count}</div>
								<div>Employees stored</div>
							</div>
							<div className="stat-card">
								<div className="stat-value">{stats.project_count}</div>
								<div>Projects tracked</div>
							</div>
							<div className="stat-card">
								<div className="stat-value">
									{formatDate(stats.created_at)}
								</div>
								<div>Database created</div>
							</div>
							<div className="stat-card">
								<div className="stat-value">
									{formatDate(stats.updated_at)}
								</div>
								<div>Last updated</div>
							</div>
						</div>
					) : (
						<div className="empty">Loading stats from actor state.</div>
					)}
				</div>

				<div className="panel">
					<div className="panel-header">
						<h3>Switch company</h3>
						<span className="count-chip">Same UI, new tenant</span>
					</div>
					<div className="form">
						<input
							value={switchName}
							onChange={(event) => setSwitchName(event.target.value)}
							placeholder="Enter a different company name"
						/>
						<button type="button" onClick={handleSwitchCompany}>
							Switch company data
						</button>
					</div>
					<div className="status-row">
						<span>Tip:</span>
						<span>
							Add employees or projects, then switch to another company.
						</span>
					</div>
				</div>
			</div>

			<div className="data-grid">
				<div className="panel">
					<div className="panel-header">
						<h3>Employees</h3>
						<span className="count-chip">{employees.length} total</span>
					</div>
					<form className="form" onSubmit={handleAddEmployee}>
						<input
							value={employeeName}
							onChange={(event) => setEmployeeName(event.target.value)}
							placeholder="Employee name"
							required
						/>
						<input
							value={employeeRole}
							onChange={(event) => setEmployeeRole(event.target.value)}
							placeholder="Role or team"
							required
						/>
						<button type="submit" disabled={!company.connection}>
							Add employee
						</button>
					</form>
					<div className="list">
						{employees.length === 0 ? (
							<div className="empty">
								No employees yet. Add a few to this company.
							</div>
						) : (
							employees.map((employee) => (
								<div key={employee.id} className="list-item">
									<strong>{employee.name}</strong>
									<span>{employee.role}</span>
									<span>
										Added {formatDate(employee.created_at)}
									</span>
								</div>
							))
						)}
					</div>
				</div>

				<div className="panel">
					<div className="panel-header">
						<h3>Projects</h3>
						<span className="count-chip">{projects.length} total</span>
					</div>
					<form className="form" onSubmit={handleAddProject}>
						<input
							value={projectName}
							onChange={(event) => setProjectName(event.target.value)}
							placeholder="Project name"
							required
						/>
						<select
							value={projectStatus}
							onChange={(event) =>
								setProjectStatus(event.target.value as ProjectStatus)
							}
						>
							{Object.entries(STATUS_LABELS).map(([value, label]) => (
								<option key={value} value={value}>
									{label}
								</option>
							))}
						</select>
						<button type="submit" disabled={!company.connection}>
							Add project
						</button>
					</form>
					<div className="list">
						{projects.length === 0 ? (
							<div className="empty">
								No projects yet. Add one to see it here.
							</div>
						) : (
							projects.map((project) => (
								<div key={project.id} className="list-item">
									<strong>{project.name}</strong>
									<span>Status: {STATUS_LABELS[project.status]}</span>
									<span>
										Added {formatDate(project.created_at)}
									</span>
								</div>
							))
						)}
					</div>
				</div>
			</div>
		</section>
	);
}
