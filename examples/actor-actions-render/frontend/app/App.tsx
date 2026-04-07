import { ActorError, createClient } from "rivetkit/client";
import { useState } from "react";
import {
	Alert,
	Badge,
	Button,
	Card,
	CardContent,
	CardHeader,
	CardTitle,
	Footer,
	FormField,
	GridDecoration,
	Navigation,
	RenderLogo,
	ThemeToggle,
} from "render-dds";
import type {
	CompanyProfile,
	EmployeeProfile,
} from "../../src/actors.ts";
import { rivetClientBase, type AppRegistry } from "./rivet-client";

const client = createClient<AppRegistry>(rivetClientBase());

export function App() {
	const [companyEin, setCompanyEin] = useState("12-3456789");
	const [companyName, setCompanyName] = useState("Acme Corp");
	const [companyIndustry, setCompanyIndustry] = useState("Technology");
	const [companyProfile, setCompanyProfile] = useState<CompanyProfile | null>(null);

	const [employeeName, setEmployeeName] = useState("Jane Smith");
	const [employeeEmail, setEmployeeEmail] = useState("jane@acme.com");
	const [employeePosition, setEmployeePosition] = useState("Software Engineer");
	const [createdEmployee, setCreatedEmployee] = useState<EmployeeProfile | null>(null);
	const [employeeList, setEmployeeList] = useState<string[]>([]);
	const [error, setError] = useState<string | null>(null);

	const createCompany = async () => {
		setError(null);
		try {
			const company = await client.company.create([companyEin], {
				input: { name: companyName, industry: companyIndustry },
			});
			setCompanyProfile(await company.getProfile());
		} catch (e) {
			if (e instanceof ActorError && e.group === "actor" && e.code === "already_exists") {
				setError(`Company with EIN "${companyEin}" already exists. Load it instead.`);
			} else throw e;
		}
	};

	const loadCompanyProfile = async () => {
		setError(null);
		const company = client.company.get([companyEin]);
		setCompanyProfile(await company.getProfile());
		setEmployeeList(await company.getEmployees());
	};

	const createEmployee = async () => {
		setError(null);
		try {
			const company = client.company.get([companyEin]);
			setCreatedEmployee(
				await company.createEmployee({
					name: employeeName,
					email: employeeEmail,
					position: employeePosition,
				}),
			);
			setEmployeeList(await company.getEmployees());
		} catch (e) {
			if (e instanceof ActorError && e.group === "actor" && e.code === "already_exists") {
				setError(`Employee "${employeeEmail}" already exists.`);
			} else throw e;
		}
	};

	const loadEmployee = async (email: string) => {
		const employee = client.employee.get([email]);
		setCreatedEmployee(await employee.getProfile());
	};

	return (
		<div className="relative flex min-h-screen flex-col bg-background text-foreground">
			<GridDecoration position="top-right" className="pointer-events-none" height={280} opacity={0.28} width={280} />
			<GridDecoration position="bottom-left" className="pointer-events-none" height={220} opacity={0.2} width={220} />

			<Navigation
				className="relative z-10 border-b border-border bg-background/80 backdrop-blur-sm"
				logo={
					<div className="flex items-center gap-3">
						<RenderLogo variant="mark" height={28} />
						<div className="flex flex-col">
							<span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">RivetKit</span>
							<span className="text-sm font-semibold leading-tight text-foreground">Actor Actions</span>
						</div>
					</div>
				}
				actions={<ThemeToggle size="sm" variant="outline" />}
			/>

			<main className="relative z-10 mx-auto flex w-full max-w-5xl flex-1 flex-col gap-6 px-4 py-10 sm:px-6">
				<div className="text-center">
					<h1 className="font-sans text-4xl font-bold tracking-tight text-foreground sm:text-5xl">
						Actor Actions
					</h1>
					<p className="mx-auto mt-4 max-w-2xl text-base text-muted-foreground sm:text-lg">
						RPC-style communication between actors and clients — create companies, hire employees, and query state with type-safe actions.
					</p>
				</div>

				{error && (
					<Alert variant="destructive" showIcon title="Error">
						<p>{error}</p>
					</Alert>
				)}

				<div className="grid gap-6 md:grid-cols-2">
					<Card variant="elevated" className="border-border shadow-lg shadow-black/5 dark:shadow-black/20">
						<div className="border-b border-border bg-muted/30 px-5 py-4 dark:bg-muted/15">
							<span className="text-sm font-semibold text-foreground">Company Actor</span>
						</div>
						<CardContent className="space-y-4 px-5 py-5">
							<FormField id="ein" label="EIN (key)" value={companyEin} onChange={(e) => setCompanyEin(e.target.value)} />
							<FormField id="company-name" label="Company Name" value={companyName} onChange={(e) => setCompanyName(e.target.value)} />
							<FormField id="industry" label="Industry" value={companyIndustry} onChange={(e) => setCompanyIndustry(e.target.value)} />
							<div className="flex gap-2 pt-2">
								<Button variant="default" className="flex-1" onClick={createCompany}>Create</Button>
								<Button variant="outline" className="flex-1" onClick={loadCompanyProfile}>Load</Button>
							</div>
						</CardContent>

						{companyProfile && (
							<div className="border-t border-border bg-muted/20 px-5 py-4 dark:bg-muted/10">
								<p className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">Profile</p>
								<dl className="space-y-1 text-sm">
									<div className="flex justify-between"><dt className="text-muted-foreground">ID</dt><dd className="font-mono text-xs">{companyProfile.id}</dd></div>
									<div className="flex justify-between"><dt className="text-muted-foreground">Name</dt><dd>{companyProfile.name}</dd></div>
									<div className="flex justify-between"><dt className="text-muted-foreground">Industry</dt><dd>{companyProfile.industry}</dd></div>
								</dl>
							</div>
						)}

						{employeeList.length > 0 && (
							<div className="border-t border-border px-5 py-4">
								<p className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
									Employees ({employeeList.length})
								</p>
								<div className="flex flex-col gap-1">
									{employeeList.map((email) => (
										<button
											key={email}
											onClick={() => loadEmployee(email)}
											className="rounded-md px-3 py-2 text-left text-sm text-primary hover:bg-muted"
										>
											{email}
										</button>
									))}
								</div>
							</div>
						)}
					</Card>

					<Card variant="elevated" className="border-border shadow-lg shadow-black/5 dark:shadow-black/20">
						<div className="border-b border-border bg-muted/30 px-5 py-4 dark:bg-muted/15">
							<span className="text-sm font-semibold text-foreground">Employee Actor</span>
							<p className="mt-0.5 text-xs text-muted-foreground">Created through the company actor</p>
						</div>
						<CardContent className="space-y-4 px-5 py-5">
							<FormField id="emp-email" label="Email (key)" type="email" value={employeeEmail} onChange={(e) => setEmployeeEmail(e.target.value)} />
							<FormField id="emp-name" label="Name" value={employeeName} onChange={(e) => setEmployeeName(e.target.value)} />
							<FormField id="emp-position" label="Position" value={employeePosition} onChange={(e) => setEmployeePosition(e.target.value)} />
							<Button variant="default" className="w-full" onClick={createEmployee}>Create via Company</Button>
						</CardContent>

						{createdEmployee && (
							<div className="border-t border-border bg-muted/20 px-5 py-4 dark:bg-muted/10">
								<p className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">Employee Profile</p>
								<dl className="space-y-1 text-sm">
									<div className="flex justify-between"><dt className="text-muted-foreground">Name</dt><dd>{createdEmployee.name}</dd></div>
									<div className="flex justify-between"><dt className="text-muted-foreground">Email</dt><dd>{createdEmployee.email}</dd></div>
									<div className="flex justify-between"><dt className="text-muted-foreground">Position</dt><dd>{createdEmployee.position}</dd></div>
									<div className="flex justify-between"><dt className="text-muted-foreground">Company</dt><dd className="font-mono text-xs">{createdEmployee.companyId}</dd></div>
								</dl>
							</div>
						)}
					</Card>
				</div>
			</main>

			<section className="flex justify-center px-4 pb-10 pt-2 md:pb-14">
				<div className="w-full max-w-md">
					<Card variant="outlined" className="border-dashed border-border/80 text-center">
						<CardHeader className="pb-2">
							<CardTitle className="text-base">Deploy on Render</CardTitle>
						</CardHeader>
						<CardContent className="flex justify-center pt-0">
							<a
								href="https://render.com/deploy?repo=https://github.com/rivet-dev/rivet/tree/main/examples/actor-actions-render"
								target="_blank"
								rel="noopener noreferrer"
								className="inline-flex shrink-0"
								aria-label="Deploy to Render"
							>
								<img
									src="https://render.com/images/deploy-to-render-button.svg"
									alt=""
									width={155}
									height={40}
									decoding="async"
								/>
							</a>
						</CardContent>
					</Card>
				</div>
			</section>

			<Footer
				centered
				className="relative z-10 mt-auto border-t border-border bg-background/90"
				copyright="actor-actions-render"
				links={[
					{ label: "Render", href: "https://render.com" },
					{ label: "Rivet", href: "https://rivet.dev" },
				]}
			/>
		</div>
	);
}
