import * as DialogPrimitive from "@radix-ui/react-dialog";
import {
	faCheck,
	faChevronDown,
	faCircleUser,
	faCreditCard,
	faGear,
	faPlusCircle,
	faRivet,
	faSparkles,
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
	Button,
	cn,
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components";
import { BillingContent } from "@/components/billing/mockup/billing-mockup";
import { MembersContent } from "@/components/mockup/members-mockup";
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

export function ProjectNamespaceSwitcher() {
	const [open, setOpen] = useState(false);
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
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					variant="ghost"
					size="sm"
					className="flex items-center gap-1.5 px-2"
					endIcon={
						<Icon
							icon={faChevronDown}
							className="text-[10px] opacity-60"
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
														"bg-accent border-l-primary aria-selected:bg-accent",
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
										onSelect={() => setOpen(false)}
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
										onSelect={() => setOpen(false)}
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
	);
}

// -- Account Menu --

type DrawerKey =
	| "billing"
	| "whats-new"
	| "settings"
	| "members"
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
	{ label: "Organization", keys: ["settings", "members", "billing"] },
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

export function MockupAccountMenu() {
	const [theme, toggleTheme] = useMockupTheme();
	const [activeDrawer, setActiveDrawer] = useState<DrawerKey | null>(null);
	const [displayedDrawer, setDisplayedDrawer] =
		useState<DrawerKey | null>(null);
	const [activeOrg, setActiveOrg] = useState<string>(MOCK_ORGS[0].name);

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
						<Icon icon={faCircleUser} className="text-base" />
						<span className="text-xs">{activeOrgData.displayName}</span>
						<Icon
							icon={faChevronDown}
							className="text-[10px] opacity-60"
						/>
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="end" className="w-52">
					<DropdownMenuItem onSelect={() => openDrawer("profile")}>
						Profile
					</DropdownMenuItem>
					<DropdownMenuItem onSelect={() => openDrawer("settings")}>
						Settings
					</DropdownMenuItem>
					<DropdownMenuItem onSelect={() => openDrawer("members")}>
						Members
					</DropdownMenuItem>
					<DropdownMenuItem onSelect={() => openDrawer("billing")}>
						Billing
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuSub>
						<DropdownMenuSubTrigger>
							Switch Organization
						</DropdownMenuSubTrigger>
						<DropdownMenuSubContent className="min-w-56">
							{MOCK_ORGS.map((org) => {
								const isActive = org.name === activeOrg;
								return (
									<DropdownMenuItem
										key={org.name}
										onSelect={() => setActiveOrg(org.name)}
										className="pl-7 relative"
									>
										{isActive ? (
											<Icon
												icon={faCheck}
												className="absolute left-2 w-3 text-muted-foreground"
											/>
										) : null}
										<span className="truncate">
											{org.displayName}
										</span>
									</DropdownMenuItem>
								);
							})}
							<DropdownMenuSeparator />
							<DropdownMenuItem className="pl-7 text-muted-foreground whitespace-nowrap">
								Create a new organization
							</DropdownMenuItem>
						</DropdownMenuSubContent>
					</DropdownMenuSub>
					<DropdownMenuSub>
						<DropdownMenuSubTrigger>Support</DropdownMenuSubTrigger>
						<DropdownMenuSubContent>
							<DropdownMenuItem
								onSelect={() => {
									window.open(
										"https://github.com/rivet-dev/rivet/issues",
										"_blank",
									);
								}}
							>
								GitHub
							</DropdownMenuItem>
							<DropdownMenuItem
								onSelect={() => {
									window.open(
										"https://rivet.dev/discord",
										"_blank",
									);
								}}
							>
								Discord
							</DropdownMenuItem>
							<DropdownMenuItem
								onSelect={() => {
									window.open(
										"https://rivet.dev/docs",
										"_blank",
									);
								}}
							>
								Documentation
							</DropdownMenuItem>
							<DropdownMenuItem>Feedback</DropdownMenuItem>
						</DropdownMenuSubContent>
					</DropdownMenuSub>
					<DropdownMenuItem onSelect={() => openDrawer("whats-new")}>
						What's new
					</DropdownMenuItem>
					<DropdownMenuItem
						onSelect={(e) => {
							e.preventDefault();
							toggleTheme();
						}}
					>
						{theme === "dark" ? "Light mode" : "Dark mode"}
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem>Sign out</DropdownMenuItem>
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
								<div className="flex items-start justify-between gap-4 px-6 pt-5 pb-4 border-b dark:border-white/10 shrink-0">
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
		</>
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
					<Icon icon={faRivet} className="text-2xl text-foreground" />
				</button>
				<div className="h-5 w-px bg-border" />
				<ProjectNamespaceSwitcher />
			</div>

			<div className="flex items-center gap-1">
				<MockupAccountMenu />
			</div>
		</div>
	);
}
