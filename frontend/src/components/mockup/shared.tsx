import * as DialogPrimitive from "@radix-ui/react-dialog";
import {
	faArrowLeft,
	faArrowRight,
	faArrowRightArrowLeft,
	faArrowRightFromBracket,
	faBookOpen,
	faBuilding,
	faCheck,
	faChevronDown,
	faCircleUser,
	faCreditCard,
	faGear,
	faLifeRing,
	faMoon,
	faPlus,
	faPlusCircle,
	faRivet,
	faSparkles,
	faSun,
	faUsers,
	faXmark,
	Icon,
} from "@rivet-gg/icons";
import { useNavigate, useParams } from "@tanstack/react-router";
import {
	useCallback,
	useEffect,
	useState,
	useSyncExternalStore,
} from "react";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	cn,
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	Dialog,
	DialogContent,
	DialogTitle,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
	Input,
	Popover,
	PopoverContent,
	PopoverTrigger,
	Textarea,
} from "@/components";
import { BillingContent } from "@/components/billing/mockup/billing-mockup";
import { MembersContent } from "@/components/mockup/members-mockup";
import { OrganizationContent } from "@/components/mockup/organization-mockup";
import { ProfileContent } from "@/components/mockup/profile-mockup";
import { SettingsContent } from "@/components/mockup/settings-mockup";

// -- Theme --

type Theme = "light" | "dark";
const THEME_STORAGE_KEY = "mockup-theme";

function readStoredTheme(): Theme {
	if (typeof window === "undefined") return "dark";
	const stored = localStorage.getItem(THEME_STORAGE_KEY);
	if (stored === "light" || stored === "dark") return stored;
	return document.documentElement.classList.contains("dark")
		? "dark"
		: "light";
}

let currentTheme: Theme =
	typeof window === "undefined" ? "dark" : readStoredTheme();
const themeListeners = new Set<() => void>();

function applyTheme(next: Theme) {
	currentTheme = next;
	document.documentElement.classList.toggle("dark", next === "dark");
	localStorage.setItem(THEME_STORAGE_KEY, next);
	for (const listener of themeListeners) listener();
}

function subscribeTheme(listener: () => void): () => void {
	themeListeners.add(listener);
	return () => {
		themeListeners.delete(listener);
	};
}

function getThemeSnapshot(): Theme {
	return currentTheme;
}

export function useMockupTheme(): [Theme, () => void] {
	const theme = useSyncExternalStore(
		subscribeTheme,
		getThemeSnapshot,
		getThemeSnapshot,
	);

	useEffect(() => {
		if (typeof window === "undefined") return;
		const isDark = document.documentElement.classList.contains("dark");
		if ((currentTheme === "dark") !== isDark) {
			applyTheme(currentTheme);
		}
	}, []);

	const toggle = useCallback(() => {
		applyTheme(currentTheme === "dark" ? "light" : "dark");
	}, []);

	return [theme, toggle];
}

// -- Static mockup data --

interface MockProject {
	name: string;
	displayName: string;
	plan: "Free" | "Hobby" | "Team" | "Enterprise";
	namespaces: { name: string; displayName: string }[];
}

const MOCK_PROJECTS: MockProject[] = [
	{
		name: "railway-test",
		displayName: "Railway test",
		plan: "Free",
		namespaces: [
			{ name: "production", displayName: "Production" },
			{ name: "staging", displayName: "Staging" },
		],
	},
	{
		name: "example-project",
		displayName: "Example Project",
		plan: "Free",
		namespaces: [{ name: "default", displayName: "Default" }],
	},
	{
		name: "empty",
		displayName: "Empty",
		plan: "Free",
		namespaces: [{ name: "default", displayName: "Default" }],
	},
	{
		name: "railway-2",
		displayName: "railway 2",
		plan: "Free",
		namespaces: [{ name: "default", displayName: "Default" }],
	},
	{
		name: "vercel-2",
		displayName: "Vercel 2",
		plan: "Free",
		namespaces: [{ name: "default", displayName: "Default" }],
	},
	{
		name: "vercel-3",
		displayName: "Vercel 3",
		plan: "Free",
		namespaces: [{ name: "default", displayName: "Default" }],
	},
	{
		name: "vercel-4",
		displayName: "Vercel 4",
		plan: "Free",
		namespaces: [{ name: "default", displayName: "Default" }],
	},
	{
		name: "vercel-5",
		displayName: "Vercel-5",
		plan: "Free",
		namespaces: [{ name: "default", displayName: "Default" }],
	},
];

// -- Project / Namespace Switcher --

function PlanBadge({ plan }: { plan: MockProject["plan"] }) {
	return (
		<span className="ml-2 shrink-0 text-[10px] font-medium uppercase tracking-wider text-muted-foreground/70">
			{plan}
		</span>
	);
}

function CreateProjectDialog({
	open,
	onOpenChange,
	onCreate,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onCreate: (name: string, orgName: string) => void;
}) {
	const [name, setName] = useState("");
	const [orgName, setOrgName] = useState<string>(MOCK_ORGS[0].name);
	const org = MOCK_ORGS.find((o) => o.name === orgName) ?? MOCK_ORGS[0];
	const trimmed = name.trim();
	const canCreate = trimmed.length > 0;

	useEffect(() => {
		if (!open) {
			setName("");
			setOrgName(MOCK_ORGS[0].name);
		}
	}, [open]);

	const handleCreate = useCallback(() => {
		if (!canCreate) return;
		onCreate(trimmed, orgName);
	}, [canCreate, onCreate, orgName, trimmed]);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				className="max-w-lg dark:border-white/10"
				hideClose
			>
				<div className="flex items-start justify-between">
					<DialogTitle className="text-lg font-semibold text-foreground">
						Create Project
					</DialogTitle>
					<DialogPrimitive.Close
						className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						aria-label="Close"
					>
						<Icon icon={faXmark} className="w-3.5" />
					</DialogPrimitive.Close>
				</div>
				<div className="space-y-4">
					<div className="space-y-1.5">
						<label
							htmlFor="create-project-org"
							className="block text-sm font-medium text-foreground"
						>
							Organization
						</label>
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<button
									id="create-project-org"
									type="button"
									className="flex w-full items-center gap-2.5 h-10 rounded-md border border-input bg-background px-3 text-left transition-colors hover:bg-accent/50 focus-visible:outline-none focus-visible:border-foreground/40"
								>
									<Avatar className="size-6 shrink-0">
										<AvatarImage
											src={`https://avatar.vercel.sh/${org.name}`}
											alt={org.displayName}
										/>
										<AvatarFallback className="text-[10px] font-medium text-foreground">
											{org.displayName[0].toUpperCase()}
										</AvatarFallback>
									</Avatar>
									<span className="flex-1 text-sm text-foreground truncate">
										{org.displayName}
									</span>
									<Icon
										icon={faChevronDown}
										className="text-[10px] text-muted-foreground"
									/>
								</button>
							</DropdownMenuTrigger>
							<DropdownMenuContent
								align="start"
								className="w-[var(--radix-dropdown-menu-trigger-width)]"
							>
								{MOCK_ORGS.map((o) => (
									<DropdownMenuItem
										key={o.name}
										onSelect={() => setOrgName(o.name)}
										className="gap-2"
									>
										<Avatar className="size-5">
											<AvatarImage
												src={`https://avatar.vercel.sh/${o.name}`}
												alt={o.displayName}
											/>
											<AvatarFallback className="text-[10px] font-medium text-foreground">
												{o.displayName[0].toUpperCase()}
											</AvatarFallback>
										</Avatar>
										<span className="truncate">
											{o.displayName}
										</span>
										{o.name === orgName ? (
											<Icon
												icon={faCheck}
												className="ml-auto w-3 text-muted-foreground"
											/>
										) : null}
									</DropdownMenuItem>
								))}
							</DropdownMenuContent>
						</DropdownMenu>
					</div>

					<div className="space-y-1.5">
						<label
							htmlFor="create-project-name"
							className="block text-sm font-medium text-foreground"
						>
							Name
						</label>
						<Input
							id="create-project-name"
							value={name}
							onChange={(e) => setName(e.target.value)}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									e.preventDefault();
									handleCreate();
								}
							}}
							placeholder="my-project"
							autoFocus
							className="h-10"
						/>
					</div>
				</div>
				<div className="flex justify-end">
					<Button
						disabled={!canCreate}
						onClick={handleCreate}
						className="bg-foreground text-background hover:bg-foreground/90"
					>
						Create
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	);
}

function slugifyName(value: string): string {
	return value
		.toLowerCase()
		.trim()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-+|-+$/g, "");
}

function CreateNamespaceDialog({
	open,
	onOpenChange,
	onCreate,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onCreate: (name: string, slug: string) => void;
}) {
	const [name, setName] = useState("");
	const [slug, setSlug] = useState("");
	const [slugTouched, setSlugTouched] = useState(false);

	const trimmedName = name.trim();
	const trimmedSlug = slug.trim();
	const canCreate = trimmedName.length > 0 && trimmedSlug.length > 0;

	useEffect(() => {
		if (!open) {
			setName("");
			setSlug("");
			setSlugTouched(false);
		}
	}, [open]);

	const handleCreate = useCallback(() => {
		if (!canCreate) return;
		onCreate(trimmedName, trimmedSlug);
	}, [canCreate, onCreate, trimmedName, trimmedSlug]);

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				className="max-w-lg dark:border-white/10"
				hideClose
			>
				<div className="flex items-start justify-between">
					<DialogTitle className="text-lg font-semibold text-foreground">
						Create Namespace
					</DialogTitle>
					<DialogPrimitive.Close
						className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						aria-label="Close"
					>
						<Icon icon={faXmark} className="w-3.5" />
					</DialogPrimitive.Close>
				</div>
				<div className="space-y-4">
					<div className="space-y-1.5">
						<label
							htmlFor="create-ns-name"
							className="block text-sm font-medium text-foreground"
						>
							Name
						</label>
						<Input
							id="create-ns-name"
							value={name}
							onChange={(e) => {
								const v = e.target.value;
								setName(v);
								if (!slugTouched) setSlug(slugifyName(v));
							}}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									e.preventDefault();
									handleCreate();
								}
							}}
							placeholder="Enter a namespace name..."
							autoFocus
							className="h-10"
						/>
					</div>
					<div className="space-y-1.5">
						<label
							htmlFor="create-ns-slug"
							className="block text-sm font-medium text-foreground"
						>
							Slug
						</label>
						<Input
							id="create-ns-slug"
							value={slug}
							onChange={(e) => {
								setSlug(e.target.value);
								setSlugTouched(true);
							}}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									e.preventDefault();
									handleCreate();
								}
							}}
							placeholder="Enter a slug..."
							className="h-10 font-mono text-sm"
						/>
					</div>
				</div>
				<div className="flex justify-end">
					<Button
						disabled={!canCreate}
						onClick={handleCreate}
						className="bg-foreground text-background hover:bg-foreground/90"
					>
						Create
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	);
}

export function ProjectNamespaceSwitcher() {
	const [open, setOpen] = useState(false);
	const [createProjectOpen, setCreateProjectOpen] = useState(false);
	const [createNamespaceOpen, setCreateNamespaceOpen] = useState(false);
	const [hoveredProject, setHoveredProject] = useState<string>(
		MOCK_PROJECTS[0].name,
	);
	const [activeProject, setActiveProject] = useState<string>(
		MOCK_PROJECTS[0].name,
	);
	const [activeNamespace, setActiveNamespace] = useState<string>(
		MOCK_PROJECTS[0].namespaces[0].name,
	);

	const activeProjectData =
		MOCK_PROJECTS.find((p) => p.name === activeProject) ?? MOCK_PROJECTS[0];
	const activeNamespaceData =
		activeProjectData.namespaces.find((ns) => ns.name === activeNamespace) ??
		activeProjectData.namespaces[0];

	const hoveredProjectData =
		MOCK_PROJECTS.find((p) => p.name === hoveredProject) ?? activeProjectData;

	return (
		<>
			<Popover open={open} onOpenChange={setOpen}>
				<PopoverTrigger asChild>
					<Button
						variant="ghost"
						size="sm"
						className="flex items-center gap-1.5 px-2 [&_svg]:size-2.5"
						endIcon={
							<Icon
								icon={faChevronDown}
								className="opacity-60"
							/>
						}
					>
						<span className="text-xs font-medium text-foreground truncate max-w-[140px]">
							{activeProjectData.displayName}
						</span>
						<span className="text-muted-foreground/40 text-xs font-normal">
							/
						</span>
						<span className="text-xs font-medium text-foreground truncate max-w-[140px]">
							{activeNamespaceData.displayName}
						</span>
				</Button>
			</PopoverTrigger>
			<PopoverContent
				className="p-0 w-[30rem] dark:border-white/10"
				align="start"
				sideOffset={6}
			>
				<div className="flex w-full">
					<div className="w-1/2 border-r dark:border-white/10">
						<Command loop>
							<div className="flex items-center border-b dark:border-white/10 h-[37px] px-3 shrink-0">
								<span className="text-xs font-medium text-foreground">
									Projects
								</span>
							</div>
							<CommandInput
								placeholder="Find project..."
								className="h-8 py-0 text-xs"
							/>
							<CommandList className="relative w-full max-h-[320px] py-1">
								<CommandGroup className="w-full p-0 [&_[cmdk-group-heading]]:hidden">
									<CommandEmpty>
										No projects found.
									</CommandEmpty>
									{MOCK_PROJECTS.map((project) => {
										const isActive =
											project.name === activeProject;
										return (
											<CommandItem
												key={project.name}
												value={project.name}
												keywords={[project.displayName]}
												className={cn(
													"static w-full h-8 rounded-none border-l-2 border-l-transparent px-3 text-xs aria-selected:bg-accent/50",
													isActive &&
														"font-medium bg-accent border-l-primary",
												)}
												onSelect={() => {
													setActiveProject(project.name);
													setActiveNamespace(
														project.namespaces[0].name,
													);
													setHoveredProject(project.name);
												}}
												onMouseEnter={() =>
													setHoveredProject(project.name)
												}
												onFocus={() =>
													setHoveredProject(project.name)
												}
											>
												<span className="truncate flex-1">
													{project.displayName}
												</span>
												<PlanBadge plan={project.plan} />
											</CommandItem>
										);
									})}
									<CommandItem
										keywords={["create", "new", "project"]}
										className="h-8 rounded-none border-l-2 border-l-transparent px-3 text-xs text-muted-foreground aria-selected:bg-accent/50"
										onSelect={() => {
											setOpen(false);
											setCreateProjectOpen(true);
										}}
									>
										<Icon
											icon={faPlusCircle}
											className="mr-1.5 w-3"
										/>
										Create Project
									</CommandItem>
								</CommandGroup>
							</CommandList>
						</Command>
					</div>
					<div className="w-1/2">
						<Command loop>
							<div className="flex items-center border-b dark:border-white/10 h-[37px] px-3 shrink-0">
								<span className="text-xs font-medium text-foreground">
									Namespaces
								</span>
							</div>
							<CommandInput
								placeholder="Find namespace..."
								className="h-8 py-0 text-xs"
							/>
							<CommandList className="relative w-full max-h-[320px] py-1">
								<CommandGroup className="w-full p-0 [&_[cmdk-group-heading]]:hidden">
									<CommandEmpty>
										No namespaces found.
									</CommandEmpty>
									{hoveredProjectData.namespaces.map(
										(namespace) => {
											const isActive =
												hoveredProjectData.name ===
													activeProject &&
												namespace.name === activeNamespace;
											return (
												<CommandItem
													key={namespace.name}
													value={namespace.name}
													keywords={[namespace.displayName]}
													className={cn(
														"static w-full h-8 rounded-none border-l-2 border-l-transparent px-3 text-xs",
														isActive &&
															"bg-accent border-l-primary",
													)}
													onSelect={() => {
														setActiveProject(
															hoveredProjectData.name,
														);
														setActiveNamespace(
															namespace.name,
														);
														setOpen(false);
													}}
												>
													<span className="truncate w-full">
														{namespace.displayName}
													</span>
												</CommandItem>
											);
										},
									)}
									<CommandItem
										keywords={[
											"create",
											"new",
											"namespace",
										]}
										className="h-8 rounded-none border-l-2 border-l-transparent px-3 text-xs text-muted-foreground aria-selected:bg-accent/50"
										onSelect={() => {
											setOpen(false);
											setCreateNamespaceOpen(true);
										}}
									>
										<Icon
											icon={faPlusCircle}
											className="mr-1.5 w-3"
										/>
										Create Namespace
									</CommandItem>
								</CommandGroup>
							</CommandList>
						</Command>
					</div>
				</div>
			</PopoverContent>
		</Popover>

			<CreateProjectDialog
				open={createProjectOpen}
				onOpenChange={setCreateProjectOpen}
				onCreate={() => {
					setCreateProjectOpen(false);
				}}
			/>
			<CreateNamespaceDialog
				open={createNamespaceOpen}
				onOpenChange={setCreateNamespaceOpen}
				onCreate={() => {
					setCreateNamespaceOpen(false);
				}}
			/>
		</>
	);
}

// -- Account Menu --

type DrawerKey =
	| "billing"
	| "whats-new"
	| "settings"
	| "members"
	| "organization"
	| "profile";

interface MockOrg {
	name: string;
	displayName: string;
}

const MOCK_ORGS: MockOrg[] = [
	{ name: "nicholas", displayName: "Nicholas" },
	{ name: "rivet", displayName: "Rivet" },
	{ name: "test-projects", displayName: "test projects" },
];

interface DrawerSpec {
	title: string;
	description?: string;
	body: React.ReactNode;
	icon: typeof faCircleUser;
}

const DRAWER_GROUPS: { label: string; keys: DrawerKey[] }[] = [
	{ label: "Account", keys: ["profile"] },
	{ label: "Project", keys: ["settings", "billing"] },
	{ label: "Organization", keys: ["organization", "members"] },
	{ label: "Other", keys: ["whats-new"] },
];

function PlaceholderBody({ message }: { message: string }) {
	return (
		<div className="flex h-full items-center justify-center py-24">
			<p className="text-sm text-muted-foreground">{message}</p>
		</div>
	);
}

const DRAWERS: Record<DrawerKey, DrawerSpec> = {
	billing: {
		title: "Billing",
		description:
			"Manage your project's billing information and view usage details.",
		body: <BillingContent />,
		icon: faCreditCard,
	},
	"whats-new": {
		title: "What's new",
		description: "Recent changes and announcements.",
		body: <PlaceholderBody message="Changelog will appear here." />,
		icon: faSparkles,
	},
	settings: {
		title: "Settings",
		description:
			"Connect your RivetKit application to Rivet Cloud. Use your cloud of choice to run Rivet Actors.",
		body: <SettingsContent />,
		icon: faGear,
	},
	organization: {
		title: "Organization",
		description: "Manage your organization.",
		body: <OrganizationContent />,
		icon: faBuilding,
	},
	members: {
		title: "Members",
		description: "Manage who has access to this organization.",
		body: <MembersContent />,
		icon: faUsers,
	},
	profile: {
		title: "Account",
		description: "Manage your account info.",
		body: <ProfileContent />,
		icon: faCircleUser,
	},
};

interface OrgPlan {
	key: string;
	name: string;
	description: string;
	features: string[];
	featured?: boolean;
}

const ORG_PLANS: OrgPlan[] = [
	{
		key: "hobby",
		name: "Hobby",
		description: "For personal projects and experiments.",
		features: [
			"Community support",
			"7-day log retention",
			"Single datacenter",
			"Basic actor limits",
			"Public projects",
		],
	},
	{
		key: "team",
		name: "Team",
		description: "For teams shipping production workloads.",
		features: [
			"Priority support",
			"30-day log retention",
			"Multi-datacenter",
			"Higher actor limits",
			"SSO & audit logs",
			"Private projects",
		],
		featured: true,
	},
	{
		key: "enterprise",
		name: "Enterprise",
		description: "Dedicated infrastructure and support.",
		features: [
			"Dedicated support",
			"Custom retention",
			"Dedicated infrastructure",
			"Custom actor limits",
			"99.99% SLA",
			"SOC 2 & custom contracts",
		],
	},
];

function PlanCard({
	plan,
	onSelect,
}: {
	plan: OrgPlan;
	onSelect: () => void;
}) {
	return (
		<div
			className={cn(
				"relative rounded-xl border dark:border-white/10 bg-card p-5 flex flex-col transition-all",
				"hover:border-foreground/20 dark:hover:border-white/20",
				plan.featured && "ring-1 ring-foreground/10 dark:ring-white/10",
			)}
		>
			{plan.featured ? (
				<span className="absolute -top-2 left-5 rounded-full border border-border bg-card text-foreground text-[10px] font-semibold tracking-wide uppercase px-2 py-0.5 shadow-sm">
					Popular
				</span>
			) : null}

			<div>
				<h3 className="text-base font-semibold text-foreground">
					{plan.name}
				</h3>
				<p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
					{plan.description}
				</p>
			</div>

			<ul className="mt-5 space-y-2 flex-1">
				{plan.features.map((feature) => (
					<li
						key={feature}
						className="flex items-start gap-2 text-xs text-muted-foreground"
					>
						<span className="mt-[5px] size-1 rounded-full shrink-0 bg-muted-foreground/50" />
						<span className="leading-relaxed">{feature}</span>
					</li>
				))}
			</ul>

			<Button
				size="sm"
				variant={plan.featured ? "default" : "outline"}
				onClick={onSelect}
				className={cn(
					"w-full mt-6",
					plan.featured &&
						"bg-foreground text-background hover:bg-foreground/90",
				)}
			>
				Select {plan.name}
			</Button>
		</div>
	);
}

type CreateOrgStep = "name" | "plan";

interface AvatarPalette {
	highlight: string;
	base: string;
	shadow: string;
}

const AVATAR_PALETTES: AvatarPalette[] = [
	{ highlight: "#fed7aa", base: "#fb923c", shadow: "#c2410c" },
	{ highlight: "#bae6fd", base: "#38bdf8", shadow: "#1d4ed8" },
	{ highlight: "#a7f3d0", base: "#34d399", shadow: "#047857" },
	{ highlight: "#f5d0fe", base: "#c084fc", shadow: "#7e22ce" },
	{ highlight: "#fecdd3", base: "#fb7185", shadow: "#be123c" },
	{ highlight: "#a5f3fc", base: "#22d3ee", shadow: "#4f46e5" },
	{ highlight: "#f5d0fe", base: "#e879f9", shadow: "#7e22ce" },
	{ highlight: "#d9f99d", base: "#a3e635", shadow: "#15803d" },
];

function getAvatarPalette(letter: string): AvatarPalette {
	const code = letter.toUpperCase().charCodeAt(0);
	if (!Number.isFinite(code) || code < 65 || code > 90) {
		return AVATAR_PALETTES[0];
	}
	return AVATAR_PALETTES[(code - 65) % AVATAR_PALETTES.length];
}

function avatarGradientStyle(palette: AvatarPalette): React.CSSProperties {
	return {
		backgroundImage: `radial-gradient(circle at 35% 30%, ${palette.highlight} 0%, ${palette.base} 45%, ${palette.shadow} 100%)`,
	};
}

function CreateOrgDialog({
	open,
	onOpenChange,
	onCreate,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onCreate: (name: string, plan: string) => void;
}) {
	const [name, setName] = useState("");
	const [step, setStep] = useState<CreateOrgStep>("name");
	const trimmed = name.trim();
	const canContinue = trimmed.length > 0;
	const initial = (trimmed[0] ?? "").toUpperCase();

	const advanceToPlan = useCallback(() => {
		if (!canContinue) return;
		setStep("plan");
	}, [canContinue]);

	const selectPlan = useCallback(
		(planKey: string) => {
			onCreate(trimmed, planKey);
			setName("");
			setStep("name");
		},
		[onCreate, trimmed],
	);

	useEffect(() => {
		if (!open) {
			setName("");
			setStep("name");
		}
	}, [open]);

	return (
		<DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
			<DialogPrimitive.Portal>
				<DialogPrimitive.Overlay
					className={cn(
						"fixed inset-0 z-[60] bg-background backdrop-blur-sm",
						"data-[state=open]:animate-in data-[state=open]:fade-in-0",
						"data-[state=closed]:animate-out data-[state=closed]:fade-out-0",
						"duration-200 ease-out",
					)}
				/>
				<DialogPrimitive.Content
					className={cn(
						"fixed inset-0 z-[70] flex flex-col outline-none bg-background",
						"data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98]",
						"data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-[0.98]",
						"duration-200 ease-out",
					)}
				>
					<div className="flex items-center justify-between px-4 pt-4 shrink-0">
						<Icon
							icon={faRivet}
							className="text-2xl text-foreground"
						/>
						<DialogPrimitive.Close
							className="rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
							aria-label="Close"
						>
							<Icon icon={faXmark} className="w-3.5" />
						</DialogPrimitive.Close>
					</div>

					{step === "name" ? (
						<>
							<div className="flex-1 flex items-center justify-center px-6 min-h-0">
								<div className="w-full max-w-sm flex flex-col items-center -mt-10">
									<DialogPrimitive.Title className="text-2xl font-semibold tracking-tight text-foreground text-center">
										Create a new organization
									</DialogPrimitive.Title>
									<DialogPrimitive.Description className="mt-2 text-sm text-muted-foreground text-center max-w-xs leading-relaxed">
										Organizations are shared workspaces
										where teammates collaborate across
										projects.
									</DialogPrimitive.Description>

									<button
										type="button"
										className="mt-10 flex flex-col items-center gap-2 group focus:outline-none"
									>
										<div
											className={cn(
												"relative size-16 rounded-full flex items-center justify-center shadow-lg transition-all overflow-hidden",
												"group-hover:ring-2 group-hover:ring-foreground/20",
												"group-focus-visible:ring-2 group-focus-visible:ring-ring",
											)}
										>
											<div
												className="absolute inset-0 animate-avatar-gradient"
												style={avatarGradientStyle(
													getAvatarPalette(initial),
												)}
											/>
											<span className="relative text-xl font-semibold text-white drop-shadow-sm">
												{initial}
											</span>
										</div>
										<span className="text-[11px] text-muted-foreground group-hover:text-foreground transition-colors">
											Choose avatar
										</span>
									</button>

									<div className="mt-8 w-full space-y-1.5">
										<label
											htmlFor="create-org-name"
											className="block text-xs font-medium text-muted-foreground"
										>
											Organization name
										</label>
										<Input
											id="create-org-name"
											value={name}
											onChange={(e) =>
												setName(e.target.value)
											}
											onKeyDown={(e) => {
												if (e.key === "Enter") {
													e.preventDefault();
													advanceToPlan();
												}
											}}
											placeholder="Acme"
											autoFocus
											className="h-9"
										/>
									</div>
								</div>
							</div>

							<div className="flex items-center justify-between gap-3 px-4 py-4 border-t dark:border-white/10 shrink-0">
								<span className="text-[11px] text-muted-foreground">
									You can change these anytime in settings.
								</span>
								<Button
									size="sm"
									disabled={!canContinue}
									onClick={advanceToPlan}
									className="h-8 gap-1.5 px-3.5 bg-foreground text-background hover:bg-foreground/90 disabled:opacity-50 disabled:cursor-not-allowed"
								>
									Continue
									<Icon
										icon={faArrowRight}
										className="w-2.5"
									/>
								</Button>
							</div>
						</>
					) : (
						<>
							<div className="flex-1 overflow-auto px-6 py-10">
								<div className="mx-auto max-w-5xl w-full flex flex-col items-center">
									<DialogPrimitive.Title className="text-2xl font-semibold tracking-tight text-foreground text-center">
										Choose your plan
									</DialogPrimitive.Title>
									<DialogPrimitive.Description className="mt-2 text-sm text-muted-foreground text-center max-w-md leading-relaxed">
										Select the plan that best fits your
										team. You can upgrade or change it
										anytime.
									</DialogPrimitive.Description>

									<div className="mt-10 w-full grid grid-cols-1 md:grid-cols-3 gap-4">
										{ORG_PLANS.map((plan) => (
											<PlanCard
												key={plan.key}
												plan={plan}
												onSelect={() =>
													selectPlan(plan.key)
												}
											/>
										))}
									</div>
								</div>
							</div>

							<div className="flex items-center justify-between gap-3 px-4 py-4 border-t dark:border-white/10 shrink-0">
								<Button
									size="sm"
									variant="ghost"
									onClick={() => setStep("name")}
									className="h-8 gap-1.5 px-3 text-muted-foreground hover:text-foreground"
								>
									<Icon
										icon={faArrowLeft}
										className="w-2.5"
									/>
									Back
								</Button>
								<span className="text-[11px] text-muted-foreground">
									Creating organization{" "}
									<span className="text-foreground font-medium">
										{trimmed || "—"}
									</span>
								</span>
							</div>
						</>
					)}
				</DialogPrimitive.Content>
			</DialogPrimitive.Portal>
		</DialogPrimitive.Root>
	);
}

function FeedbackBody({ onDone }: { onDone: () => void }) {
	const [value, setValue] = useState("");
	const canSend = value.trim().length > 0;

	const handleSend = useCallback(() => {
		if (!canSend) return;
		onDone();
	}, [canSend, onDone]);

	return (
		<>
			<div className="p-3">
				<Textarea
					value={value}
					onChange={(e) => setValue(e.target.value)}
					onKeyDown={(e) => {
						if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
							e.preventDefault();
							handleSend();
						}
					}}
					placeholder="Have an idea to improve the product? Tell the Rivet team..."
					className="min-h-[120px] resize-none border-0 bg-muted/40 p-3 text-sm focus-visible:border-0 focus-within:border-0"
					autoFocus
				/>
			</div>
			<div className="flex items-center justify-between gap-3 px-4 pb-3">
				<p className="text-xs text-muted-foreground">
					Need help?{" "}
					<button
						type="button"
						onClick={() => {
							window.open(
								"https://rivet.dev/discord",
								"_blank",
							);
						}}
						className="text-foreground underline-offset-2 hover:underline"
					>
						Join Discord
					</button>{" "}
					or{" "}
					<button
						type="button"
						onClick={() => {
							window.open("https://rivet.dev/docs", "_blank");
						}}
						className="text-foreground underline-offset-2 hover:underline"
					>
						see docs
					</button>
					.
				</p>
				<Button
					size="sm"
					disabled={!canSend}
					onClick={handleSend}
					className="h-7 gap-1.5 pl-2.5 pr-1.5 bg-foreground text-background hover:bg-foreground/90 disabled:opacity-50"
				>
					<span className="text-xs">Send</span>
					<span className="flex items-center gap-0.5">
						<kbd className="inline-flex items-center justify-center rounded bg-background/20 text-background px-1 h-4 text-[10px] font-sans">
							⌘
						</kbd>
						<kbd className="inline-flex items-center justify-center rounded bg-background/20 text-background px-1 h-4 text-[10px] font-sans">
							↵
						</kbd>
					</span>
				</Button>
			</div>
		</>
	);
}

export function MockupAccountMenu() {
	const [theme, toggleTheme] = useMockupTheme();
	const [activeDrawer, setActiveDrawer] = useState<DrawerKey | null>(null);
	const [displayedDrawer, setDisplayedDrawer] =
		useState<DrawerKey | null>(null);
	const [activeOrg, setActiveOrg] = useState<string>(MOCK_ORGS[0].name);
	const [createOrgOpen, setCreateOrgOpen] = useState(false);

	const openDrawer = useCallback((key: DrawerKey) => {
		setDisplayedDrawer(key);
		setActiveDrawer(key);
	}, []);

	const spec = displayedDrawer ? DRAWERS[displayedDrawer] : null;
	const activeOrgData =
		MOCK_ORGS.find((o) => o.name === activeOrg) ?? MOCK_ORGS[0];

	return (
		<>
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button
						variant="ghost"
						size="sm"
						className="text-muted-foreground gap-2"
					>
						<Avatar className="size-5">
							<AvatarImage
								src={`https://avatar.vercel.sh/${activeOrgData.name}`}
								alt={activeOrgData.displayName}
							/>
							<AvatarFallback className="text-[10px] font-medium text-foreground">
								{activeOrgData.displayName[0].toUpperCase()}
							</AvatarFallback>
						</Avatar>
						<span className="text-xs">{activeOrgData.displayName}</span>
						<Icon
							icon={faChevronDown}
							className="text-[10px] opacity-60"
						/>
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="end" className="w-52">
					<DropdownMenuItem
						onSelect={() => openDrawer("profile")}
						className="gap-2"
					>
						<Icon
							icon={faCircleUser}
							className="w-3.5 text-muted-foreground"
						/>
						Profile
					</DropdownMenuItem>
					<DropdownMenuItem
						onSelect={() => openDrawer("settings")}
						className="gap-2"
					>
						<Icon
							icon={faGear}
							className="w-3.5 text-muted-foreground"
						/>
						Settings
					</DropdownMenuItem>
					<DropdownMenuItem
						onSelect={() => openDrawer("billing")}
						className="gap-2"
					>
						<Icon
							icon={faCreditCard}
							className="w-3.5 text-muted-foreground"
						/>
						Billing
					</DropdownMenuItem>
					<DropdownMenuItem
						onSelect={() => openDrawer("members")}
						className="gap-2"
					>
						<Icon
							icon={faUsers}
							className="w-3.5 text-muted-foreground"
						/>
						Members
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuSub>
						<DropdownMenuSubTrigger className="gap-2">
							<Icon
								icon={faArrowRightArrowLeft}
								className="w-3.5 text-muted-foreground"
							/>
							Switch Organization
						</DropdownMenuSubTrigger>
						<DropdownMenuSubContent className="min-w-56">
							{MOCK_ORGS.map((org) => {
								const isActive = org.name === activeOrg;
								return (
									<DropdownMenuItem
										key={org.name}
										onSelect={() => setActiveOrg(org.name)}
										className="pl-7 gap-2 relative"
									>
										{isActive ? (
											<Icon
												icon={faCheck}
												className="absolute left-2 w-3 text-muted-foreground"
											/>
										) : null}
										<Avatar className="size-5 shrink-0">
											<AvatarImage
												src={`https://avatar.vercel.sh/${org.name}`}
												alt={org.displayName}
											/>
											<AvatarFallback className="text-[10px] font-medium text-foreground">
												{org.displayName[0].toUpperCase()}
											</AvatarFallback>
										</Avatar>
										<span className="truncate">
											{org.displayName}
										</span>
									</DropdownMenuItem>
								);
							})}
							<DropdownMenuSeparator />
							<DropdownMenuItem
								onSelect={() => setCreateOrgOpen(true)}
								className="pl-7 text-muted-foreground whitespace-nowrap relative"
							>
								<Icon
									icon={faPlus}
									className="absolute left-2 w-3"
								/>
								Create a new organization
							</DropdownMenuItem>
						</DropdownMenuSubContent>
					</DropdownMenuSub>
					<DropdownMenuItem
						onSelect={() => openDrawer("whats-new")}
						className="gap-2"
					>
						<Icon
							icon={faSparkles}
							className="w-3.5 text-muted-foreground"
						/>
						What's new
					</DropdownMenuItem>
					<DropdownMenuItem
						onSelect={(e) => {
							e.preventDefault();
							toggleTheme();
						}}
						className="gap-2"
					>
						<Icon
							icon={theme === "dark" ? faSun : faMoon}
							className="w-3.5 text-muted-foreground"
						/>
						{theme === "dark" ? "Light mode" : "Dark mode"}
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem className="gap-2">
						<Icon
							icon={faArrowRightFromBracket}
							className="w-3.5 text-muted-foreground"
						/>
						Sign out
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>

			<DialogPrimitive.Root
				open={activeDrawer !== null}
				onOpenChange={(open) => {
					if (!open) setActiveDrawer(null);
				}}
			>
				{spec ? (
					<DialogPrimitive.Portal>
						<DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-black/30 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 duration-200 ease-out" />
						<DialogPrimitive.Content
							className={cn(
								"fixed top-16 inset-x-2 bottom-2 z-50 flex rounded-lg border dark:border-white/10 bg-card shadow-xl overflow-hidden",
								"data-[state=open]:animate-in data-[state=open]:slide-in-from-right-4 data-[state=open]:fade-in-0",
								"data-[state=closed]:animate-out data-[state=closed]:slide-out-to-right-4 data-[state=closed]:fade-out-0",
								"duration-200 ease-out",
							)}
						>
							<aside className="w-56 shrink-0 border-r dark:border-white/10 p-3 overflow-y-auto">
								<div className="px-2 pt-2 pb-3">
									<div className="text-xs font-semibold text-foreground">
										{activeOrgData.displayName}
									</div>
								</div>
								{DRAWER_GROUPS.map((group) => (
									<div key={group.label} className="mb-3">
										<div className="px-2 pb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
											{group.label}
										</div>
										<nav className="flex flex-col gap-0.5">
											{group.keys.map((key) => {
												const d = DRAWERS[key];
												const isActive =
													activeDrawer === key;
												return (
													<button
														key={key}
														type="button"
														onClick={() =>
															openDrawer(key)
														}
														className={cn(
															"flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm text-left transition-colors",
															isActive
																? "bg-accent text-foreground"
																: "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
														)}
													>
														<Icon
															icon={d.icon}
															className="w-3.5 shrink-0"
														/>
														<span className="truncate">
															{d.title}
														</span>
													</button>
												);
											})}
										</nav>
									</div>
								))}
							</aside>
							<div className="flex-1 flex flex-col min-w-0">
								<div className="flex items-start justify-between gap-4 px-6 pt-5 pb-2 shrink-0">
									<div>
										<DialogPrimitive.Title className="text-lg font-semibold text-foreground leading-tight">
											{spec.title}
										</DialogPrimitive.Title>
										{spec.description ? (
											<DialogPrimitive.Description className="mt-0.5 text-xs text-muted-foreground">
												{spec.description}
											</DialogPrimitive.Description>
										) : null}
									</div>
									<DialogPrimitive.Close
										className="rounded-sm text-muted-foreground opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
										aria-label="Close"
									>
										<Icon icon={faXmark} className="h-4 w-4" />
									</DialogPrimitive.Close>
								</div>
								<div className="flex-1 overflow-auto">
									<div className="mx-auto max-w-5xl px-6 pt-6">
										{spec.body}
									</div>
								</div>
							</div>
						</DialogPrimitive.Content>
					</DialogPrimitive.Portal>
				) : null}
			</DialogPrimitive.Root>

			<CreateOrgDialog
				open={createOrgOpen}
				onOpenChange={setCreateOrgOpen}
				onCreate={(_name, _plan) => {
					setCreateOrgOpen(false);
				}}
			/>
		</>
	);
}

// -- Feedback Button --

function FeedbackButton() {
	const [open, setOpen] = useState(false);

	useEffect(() => {
		const handler = (e: KeyboardEvent) => {
			if (e.key !== "f" && e.key !== "F") return;
			if (e.metaKey || e.ctrlKey || e.altKey) return;
			const target = e.target as HTMLElement | null;
			if (
				target &&
				(target.tagName === "INPUT" ||
					target.tagName === "TEXTAREA" ||
					target.isContentEditable)
			)
				return;
			e.preventDefault();
			setOpen((prev) => !prev);
		};
		window.addEventListener("keydown", handler);
		return () => window.removeEventListener("keydown", handler);
	}, []);

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground hover:text-foreground gap-1.5 pr-1.5"
				>
					<span className="text-xs">Feedback</span>
					<kbd className="inline-flex items-center justify-center h-[18px] min-w-[18px] px-1 rounded bg-muted text-[10px] font-sans text-muted-foreground">
						F
					</kbd>
				</Button>
			</PopoverTrigger>
			<PopoverContent
				align="end"
				sideOffset={8}
				className="w-[400px] p-0 dark:border-white/10"
			>
				<FeedbackBody onDone={() => setOpen(false)} />
			</PopoverContent>
		</Popover>
	);
}

// -- Help --

type HelpImpact = "general" | "minor" | "major" | "critical";

const HELP_IMPACT_LABELS: Record<HelpImpact, string> = {
	general: "General question",
	minor: "Minor — non-blocking issue",
	major: "Major — blocking my work",
	critical: "Critical — production down",
};

function HelpBody({ onDone }: { onDone: () => void }) {
	const [subject, setSubject] = useState("");
	const [message, setMessage] = useState("");
	const [impact, setImpact] = useState<HelpImpact>("general");
	const canSend =
		subject.trim().length > 0 && message.trim().length > 0;

	const handleSend = useCallback(() => {
		if (!canSend) return;
		onDone();
	}, [canSend, onDone]);

	return (
		<>
			<div className="flex items-center justify-between px-4 py-2.5 border-b dark:border-white/10">
				<h3 className="text-sm font-semibold text-foreground">
					Contact support
				</h3>
				<a
					href="https://status.rivet.dev"
					target="_blank"
					rel="noreferrer"
					className="flex items-center gap-1.5 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
				>
					<span className="inline-flex size-1.5 rounded-full bg-emerald-500" />
					All systems operational
				</a>
			</div>
			<div className="p-3 space-y-3">
				<div className="space-y-1.5">
					<label
						htmlFor="help-subject"
						className="block text-xs font-medium text-foreground"
					>
						Subject
					</label>
					<Input
						id="help-subject"
						value={subject}
						onChange={(e) => setSubject(e.target.value)}
						placeholder="Briefly describe the issue"
						className="h-9 text-sm"
					/>
				</div>
				<div className="space-y-1.5">
					<label
						htmlFor="help-message"
						className="block text-xs font-medium text-foreground"
					>
						Message
					</label>
					<Textarea
						id="help-message"
						value={message}
						onChange={(e) => setMessage(e.target.value)}
						placeholder="Provide as much detail as possible..."
						className="min-h-[120px] resize-none text-sm"
					/>
				</div>
				<div className="space-y-1.5">
					<span className="block text-xs font-medium text-foreground">
						Impact
					</span>
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<button
								type="button"
								className="flex w-full items-center justify-between h-9 px-3 rounded-md border border-input bg-background text-sm text-foreground transition-colors hover:bg-accent/50 focus-visible:outline-none focus-visible:border-foreground/40"
							>
								<span>{HELP_IMPACT_LABELS[impact]}</span>
								<Icon
									icon={faChevronDown}
									className="text-[10px] text-muted-foreground"
								/>
							</button>
						</DropdownMenuTrigger>
						<DropdownMenuContent
							align="start"
							className="w-[var(--radix-dropdown-menu-trigger-width)]"
						>
							{(
								Object.keys(HELP_IMPACT_LABELS) as HelpImpact[]
							).map((k) => (
								<DropdownMenuItem
									key={k}
									onSelect={() => setImpact(k)}
								>
									{HELP_IMPACT_LABELS[k]}
								</DropdownMenuItem>
							))}
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</div>
			<div className="flex items-center justify-between gap-3 px-4 pb-3">
				<p className="text-xs text-muted-foreground">
					Prefer the community?{" "}
					<a
						href="https://rivet.dev/discord"
						target="_blank"
						rel="noreferrer"
						className="text-foreground underline-offset-2 hover:underline"
					>
						Discord
					</a>{" "}
					·{" "}
					<a
						href="https://github.com/rivet-dev/rivet"
						target="_blank"
						rel="noreferrer"
						className="text-foreground underline-offset-2 hover:underline"
					>
						GitHub
					</a>
				</p>
				<Button
					size="sm"
					disabled={!canSend}
					onClick={handleSend}
					className="h-7 px-3 bg-foreground text-background hover:bg-foreground/90 disabled:opacity-50"
				>
					<span className="text-xs">Send</span>
				</Button>
			</div>
		</>
	);
}

function HelpButton() {
	const [open, setOpen] = useState(false);

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground hover:text-foreground gap-1.5"
					aria-label="Help"
				>
					<Icon icon={faLifeRing} className="w-3" />
					<span className="text-xs">Help</span>
				</Button>
			</PopoverTrigger>
			<PopoverContent
				align="end"
				sideOffset={8}
				className="w-[400px] p-0 dark:border-white/10"
			>
				<HelpBody onDone={() => setOpen(false)} />
			</PopoverContent>
		</Popover>
	);
}

// -- Top Bar --

export function MockupTopBar() {
	const navigate = useNavigate();
	const { namespace } = useParams({ strict: false }) as {
		namespace?: string;
	};
	const ns = namespace ?? "default";

	return (
		<div className="h-12 mt-2 mx-2 border dark:border-white/10 rounded-lg bg-card flex items-center justify-between px-3 shrink-0 z-20">
			<div className="flex items-center gap-3">
				<button
					type="button"
					onClick={() =>
						navigate({
							to: "/ns/$namespace/mockup",
							params: { namespace: ns },
						})
					}
					className="flex items-center"
					aria-label="Home"
				>
					<Icon
						icon={faRivet}
						className="text-2xl text-foreground/60 dark:text-foreground"
					/>
				</button>
				<div className="h-5 w-px bg-border" />
				<ProjectNamespaceSwitcher />
			</div>

			<div className="flex items-center gap-2">
				<FeedbackButton />
				<HelpButton />
				<Button
					variant="ghost"
					size="sm"
					className="text-muted-foreground hover:text-foreground gap-1.5"
					onClick={() => window.open("https://rivet.dev/docs", "_blank")}
				>
					<Icon icon={faBookOpen} className="w-3" />
					<span className="text-xs">Docs</span>
				</Button>
				<MockupAccountMenu />
			</div>
		</div>
	);
}
