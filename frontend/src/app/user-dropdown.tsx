import {
	faArrowRightFromBracket,
	faChevronDown,
	faCreditCard,
	faGear,
	faMoon,
	faPlus,
	faRightLeft,
	faSparkles,
	faSun,
	faUserCircle,
	faUsers,
	Icon,
} from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useMatchRoute, useNavigate, useParams, useRouter } from "@tanstack/react-router";
import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuPortal,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
	Skeleton,
} from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { authClient } from "@/lib/auth";
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
import { useTheme } from "@/lib/theme";
import { queryClient } from "@/queries/global";

export function UserDropdown({ children }: { children?: React.ReactNode }) {
	const params = useParams({
		strict: false,
	});
	const router = useRouter();

	const { data: session } = authClient.useSession();
	const navigate = useNavigate();
	const match = useMatchRoute();
	const { theme, toggle: toggleTheme } = useTheme();

	const isMatchingProjectRoute = match({
		to: "/orgs/$organization/projects/$project",
		fuzzy: true,
	});

	const goToBilling = () => {
		if (isMatchingProjectRoute) {
			return navigate({
				to: ".",
				search: (old) => ({ ...old, modal: "billing" }),
			});
		}
		return navigate({
			to: ".",
			search: (old) => ({ ...old, modal: "billing" }),
		});
	};

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild={!params.organization || !!children}>
				{children ||
					(params.organization ? (
						<Preview org={params.organization} />
					) : (
						<Button
							variant="ghost"
							size="xs"
							className="text-muted-foreground justify-between py-1 min-h-8 gap-2 w-full"
							endIcon={<Icon icon={faChevronDown} />}
						>
							{session?.user?.email}
						</Button>
					))}
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="min-w-56">
				<DropdownMenuItem
					onSelect={() => {
						return navigate({
							to: ".",
							search: (old) => ({ ...old, modal: "profile" }),
						});
					}}
				>
					<Icon icon={faUserCircle} className="mr-2 size-3.5 text-muted-foreground" />
					Profile
				</DropdownMenuItem>
				<DropdownMenuItem
					onSelect={() => {
						return navigate({
							to: ".",
							search: (old) => ({
								...old,
								modal: "settings",
							}),
						});
					}}
				>
					<Icon icon={faGear} className="mr-2 size-3.5 text-muted-foreground" />
					Settings
				</DropdownMenuItem>
				{isMatchingProjectRoute ? (
					<DropdownMenuItem onSelect={goToBilling}>
						<Icon icon={faCreditCard} className="mr-2 size-3.5 text-muted-foreground" />
						Billing
					</DropdownMenuItem>
				) : null}
				{params.organization ? (
					<DropdownMenuItem
						onSelect={() => {
							return navigate({
								to: ".",
								search: (old) => ({
									...old,
									modal: "members",
								}),
							});
						}}
					>
						<Icon icon={faUsers} className="mr-2 size-3.5 text-muted-foreground" />
						Members
					</DropdownMenuItem>
				) : null}
				<DropdownMenuSeparator />
				{params.organization ? (
					<DropdownMenuSub>
						<DropdownMenuSubTrigger>
							<Icon icon={faRightLeft} className="mr-2 size-3.5 text-muted-foreground" />
							Switch Organization
						</DropdownMenuSubTrigger>
						<DropdownMenuPortal>
							<DropdownMenuSubContent
								sideOffset={8}
								className="min-w-56"
							>
								<OrganizationSwitcher
									value={params.organization}
								/>
							</DropdownMenuSubContent>
						</DropdownMenuPortal>
					</DropdownMenuSub>
				) : null}
				<DropdownMenuItem
					onSelect={() => {
						window.open(
							"https://www.rivet.dev/changelog",
							"_blank",
							"noopener,noreferrer",
						);
					}}
				>
					<Icon icon={faSparkles} className="mr-2 size-3.5 text-muted-foreground" />
					What's new
				</DropdownMenuItem>
				<DropdownMenuItem
					onSelect={(e) => {
						e.preventDefault();
						toggleTheme();
					}}
				>
					<Icon
						icon={theme === "dark" ? faSun : faMoon}
						className="mr-2 size-3.5 text-muted-foreground"
					/>
					{theme === "dark" ? "Light mode" : "Dark mode"}
				</DropdownMenuItem>
				<DropdownMenuSeparator />
				<DropdownMenuItem
					onSelect={() => {
						authClient.signOut();
						router.invalidate();
						queryClient.clear();
						return navigate({ to: "/login" });
					}}
				>
					<Icon icon={faArrowRightFromBracket} className="mr-2 size-3.5 text-muted-foreground" />
					Sign out
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

function Preview({ org }: { org: string }) {
	const { isLoading, data } = useQuery(
		useCloudDataProvider().organizationQueryOptions({ org }),
	);

	return (
		<Button
			variant="ghost"
			size="xs"
			className="text-muted-foreground justify-between py-1 min-h-8 gap-2 w-full"
			endIcon={<Icon icon={faChevronDown} />}
		>
			<div className="flex gap-2 items-center w-full min-w-0">
				<Avatar className="size-5">
					<AvatarImage src={data?.logo ?? undefined} />
					<AvatarFallback
						className="text-white text-[10px] font-semibold"
						style={
							data?.name
								? {
										backgroundImage: orgConicGradient(
											paletteForLetter(data.name),
										),
									}
								: undefined
						}
					>
						{isLoading ? (
							<Skeleton className="h-5 w-5" />
						) : (
							data?.name[0].toUpperCase()
						)}
					</AvatarFallback>
				</Avatar>
				<span className="text-sm truncate">
					{isLoading ? (
						<Skeleton className="w-full h-4 flex-1" />
					) : (
						data?.name
					)}
				</span>
			</div>
		</Button>
	);
}

function OrganizationSwitcher({ value }: { value: string | undefined }) {
	const { data: organizations, isPending: isLoading } =
		authClient.useListOrganizations();

	const navigate = useNavigate();

	return (
		<>
			{isLoading ? (
				<>
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
					<DropdownMenuCheckboxItem>
						<Skeleton className="h-4 w-full" />
					</DropdownMenuCheckboxItem>
				</>
			) : null}
			{organizations?.map((org) => (
				<DropdownMenuCheckboxItem
					key={org.id}
					checked={org.slug === value}
					onSelect={() => {
						// Don't call `setActive` here — the org route's
						// `beforeLoad` handles it once based on the URL params.
						// Calling it eagerly races with the route transition
						// and fires Better Auth updates against components that
						// are mounting/unmounting, causing the
						// "state update on unmounted component" warning.
						navigate({
							to: `/orgs/$organization`,
							params: {
								organization: org.slug,
							},
						});
					}}
				>
					<Avatar className="size-6 mr-2">
						<AvatarImage src={org.logo ?? undefined} />
						<AvatarFallback
							className="text-white text-[11px] font-semibold"
							style={{
								backgroundImage: orgConicGradient(
									paletteForLetter(org.name),
								),
							}}
						>
							{org.name[0].toUpperCase()}
						</AvatarFallback>
					</Avatar>
					{org.name}
				</DropdownMenuCheckboxItem>
			))}
			<DropdownMenuItem
				onSelect={() => {
					navigate({ to: "/new-org" });
				}}
				indicator={<Icon icon={faPlus} className="size-4 text-muted-foreground" />}
			>
				Create a new organization
			</DropdownMenuItem>
		</>
	);
}
