import {
	Button,
	CommandDialog,
	CommandEmpty,
	CommandInput,
	CommandList,
	CommandLoading,
	Kbd,
	ShimmerLine,
	cn,
} from "@rivet-gg/components";
import { useIsFetching } from "@tanstack/react-query";
import { useMatchRoute } from "@tanstack/react-router";
import {
	type KeyboardEventHandler,
	Suspense,
	startTransition,
	useCallback,
	useEffect,
	useState,
} from "react";
import { CommandPanelNavigationBreadcrumbs } from "./command-panel/command-panel-navigation-breadcrumbs";
import {
	CommandPanelNavigationProvider,
	type CommandPanelPage,
} from "./command-panel/command-panel-navigation-provider";
import { EnvironmentCommandPanelPage } from "./command-panel/command-panel-page/environment-command-panel-page";
import { GroupCommandPanelPage } from "./command-panel/command-panel-page/group-command-panel-page";
import { IndexCommandPanelPage } from "./command-panel/command-panel-page/index-command-panel-page";
import { ProjectCommandPanelPage } from "./command-panel/command-panel-page/project-command-panel-page";

export function CommandPanel({ className }: { className?: string }) {
	const [isOpen, setOpen] = useState(false);

	const [search, setSearch] = useState("");
	const [pages, setPages] = useState<CommandPanelPage[]>([]);
	const page = pages[pages.length - 1];
	const matchRoute = useMatchRoute();

	// biome-ignore lint/correctness/useExhaustiveDependencies: we do not want to run this effect on every change of match route
	const open = useCallback(() => {
		startTransition(() => {
			const isTeam = matchRoute({
				to: "/teams/$groupId",
				fuzzy: true,
			}) as { groupId: string } | false;

			if (isTeam) {
				setPages([
					{ key: "group", params: { groupId: isTeam.groupId } },
				]);
			}

			const isProject = matchRoute({
				to: "/projects/$projectNameId",
				fuzzy: true,
			});
			if (isProject) {
				setPages([
					{
						key: "project",
						params: { projectNameId: isProject.projectNameId },
					},
				]);
			}

			const isEnvironment = matchRoute({
				to: "/projects/$projectNameId/environments/$environmentNameId",
				fuzzy: true,
			});
			if (isEnvironment) {
				setPages([
					{
						key: "project",
						params: { projectNameId: isEnvironment.projectNameId },
					},
					{
						key: "environment",
						params: {
							projectNameId: isEnvironment.projectNameId,
							environmentNameId: isEnvironment.environmentNameId,
						},
					},
				]);
			}

			setOpen((open) => !open);
		});
	}, []);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we do not want to run this effect on every change of match route
	useEffect(() => {
		const down = (e: KeyboardEvent) => {
			if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
				e.preventDefault();
				open();
			}
		};
		document.addEventListener("keydown", down);
		return () => document.removeEventListener("keydown", down);
	}, []);

	const handlePageChange = useCallback((page: CommandPanelPage) => {
		startTransition(() => {
			setPages((pages) => [...pages, page]);
			setSearch("");
		});
	}, []);

	const handleClose = useCallback(() => {
		startTransition(() => {
			setOpen(false);
			setSearch("");
			setPages([]);
		});
	}, []);

	const handleKeyDown: KeyboardEventHandler<HTMLDivElement> = useCallback(
		(e) => {
			// Escape goes to previous page
			// Backspace goes to previous page when search is empty
			if (
				(e.key === "Escape" || (e.key === "Backspace" && !search)) &&
				pages.length > 0
			) {
				e.preventDefault();
				setPages((pages) => pages.slice(0, -1));
			}
		},
		[pages.length, search],
	);

	const isLoading =
		useIsFetching({
			predicate: (query) => !query.queryKey.includes("watch"),
		}) > 0;

	return (
		<>
			<Button
				onClick={open}
				variant="outline"
				className={cn(
					"relative h-8 w-full justify-start rounded-[0.5rem] bg-background text-sm font-normal text-muted-foreground shadow-none hidden md:flex md:w-40 lg:w-64",
					className,
				)}
			>
				<span className="hidden lg:inline-flex">Search...</span>
				<span className="inline-flex lg:hidden">Search...</span>
				<Kbd className="absolute right-[0.3rem] top-[0.3rem] hidden sm:flex">
					<Kbd.Key />K
				</Kbd>
			</Button>
			<CommandDialog
				commandProps={{
					onKeyDown: handleKeyDown,
					shouldFilter: !isLoading,
				}}
				open={isOpen}
				onOpenChange={setOpen}
			>
				<CommandPanelNavigationBreadcrumbs pages={pages} />
				<CommandInput
					value={search}
					onValueChange={setSearch}
					placeholder="Type a command or search..."
				/>
				<CommandPanelNavigationProvider
					isLoading={isLoading}
					onClose={handleClose}
					onChangePage={handlePageChange}
				>
					<CommandList>
						<Suspense
							fallback={<CommandLoading>Hang on…</CommandLoading>}
						>
							{isLoading ? (
								<ShimmerLine className="-top-[1px]" />
							) : null}
							<CommandEmpty>No results found.</CommandEmpty>
							{!page ? <IndexCommandPanelPage /> : null}
							{page?.key === "group" ? (
								<GroupCommandPanelPage
									groupId={page.params.groupId}
								/>
							) : null}
							{page?.key === "project" ? (
								<ProjectCommandPanelPage
									projectNameId={page.params.projectNameId}
								/>
							) : null}
							{page?.key === "environment" ? (
								<EnvironmentCommandPanelPage
									projectNameId={page.params.projectNameId}
									environmentNameId={
										page.params.environmentNameId
									}
								/>
							) : null}
						</Suspense>
					</CommandList>
				</CommandPanelNavigationProvider>
			</CommandDialog>
		</>
	);
}
