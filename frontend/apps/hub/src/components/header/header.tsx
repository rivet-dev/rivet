import { useAuth } from "@/domains/auth/contexts/auth";
import { publicUrl } from "@/lib/utils";
import {
	Button,
	Flex,
	Sheet,
	SheetContent,
	SheetTrigger,
	cn,
} from "@rivet-gg/components";
import { Icon, faBars } from "@rivet-gg/icons";
import { ErrorBoundary } from "@sentry/react";
import { Link } from "@tanstack/react-router";
import { Breadcrumbs } from "../breadcrumbs/breadcrumbs";
import { MobileBreadcrumbs } from "../breadcrumbs/mobile-breadcrumbs";
import { Changelog } from "./changelog";
import { HeaderRouteLoader } from "./header-route-loader";
import { HeaderSubNav } from "./header-sub-nav";
import { MobileHeaderSubNav } from "./mobile-header-sub-nav";
import { NavItem } from "./nav-item";

const UserProfileButton = () => {
	const { profile } = useAuth();
	if (!profile?.identity.isRegistered) {
		return null;
	}
	return (
		<Button
			asChild
			variant="secondary"
			size="icon"
			className="rounded-full"
		>
			<Link to="/my-profile">
				<img
					src={profile.identity.avatarUrl}
					alt="User avatar"
					className="size-7 rounded-full"
				/>
			</Link>
		</Button>
	);
};

interface HeaderProps {
	variant: "default" | "opaque";
}

export function Header({ variant = "opaque" }: HeaderProps) {
	return (
		<header
			className={cn(
				" sticky top-0 z-10 flex items-center gap-4 border-b py-2",
				{
					"bg-background/60 backdrop-blur": variant === "default",
					"bg-background": variant === "opaque",
				},
			)}
		>
			<HeaderRouteLoader />
			<div className="w-full px-8 flex min-h-10 flex-col justify-center gap-4">
				<div className="flex w-full items-center gap-4">
					<Sheet>
						<SheetTrigger asChild>
							<Button
								variant="outline"
								size="icon"
								className="shrink-0 md:hidden"
							>
								<Icon icon={faBars} className="size-5" />
								<span className="sr-only">
									Toggle navigation menu
								</span>
							</Button>
						</SheetTrigger>
						<SheetContent side="left">
							<nav className="grid min-h-full gap-6 text-lg font-medium">
								<div className="flex-1">
									<Flex direction="col" gap="6">
										<Link
											href="/"
											className="flex items-center gap-2 text-lg font-semibold"
										>
											<img
												className="h-6"
												src={publicUrl(
													"/icon-white-borderless.svg",
												)}
												alt="Rivet logo"
											/>
										</Link>
										<MobileBreadcrumbs />
										<MobileHeaderSubNav />
									</Flex>
								</div>
								<Flex direction="col" justify="end" gap="6">
									<NavItem asChild>
										<Link
											to={"."}
											search={{ modal: "feedback" }}
										>
											Feedback
										</Link>
									</NavItem>
									<NavItem asChild>
										<a
											href="https://rivet.gg/changelog"
											target="_blank"
											rel="noreferrer"
										>
											Changelog
										</a>
									</NavItem>
									<NavItem asChild>
										<a
											href="https://rivet.gg/support"
											target="_blank"
											rel="noreferrer"
										>
											Help
										</a>
									</NavItem>
									<NavItem asChild>
										<a
											href="https://rivet.gg/docs"
											target="_blank"
											rel="noreferrer"
										>
											Docs
										</a>
									</NavItem>
								</Flex>
							</nav>
						</SheetContent>
					</Sheet>
					<nav className="flex-1 font-medium md:flex md:flex-row md:items-center md:gap-5 md:text-sm lg:gap-6">
						<Link to="/">
							<img
								className="h-6"
								src={publicUrl("/icon-white-borderless.svg")}
								alt="Rivet logo"
							/>
						</Link>
						<div className="hidden md:flex">
							<Breadcrumbs />
						</div>
					</nav>
					<div className="gap-6 font-medium md:flex md:flex-row md:items-center md:gap-5 md:text-sm">
						<NavItem
							asChild
							className="hidden md:inline-block border px-4 py-2 rounded-md"
						>
							<Link to="." search={{ modal: "feedback" }}>
								Feedback
							</Link>
						</NavItem>
						<ErrorBoundary
							fallback={
								<NavItem
									asChild
									className="hidden md:inline-block"
								>
									<a
										href="https://rivet.gg/changelog"
										target="_blank"
										rel="noreferrer"
									>
										Changelog
									</a>
								</NavItem>
							}
						>
							<Changelog />
						</ErrorBoundary>
						<NavItem asChild className="hidden md:inline-block">
							<a
								href="https://rivet.gg/support"
								target="_blank"
								rel="noreferrer"
							>
								Help
							</a>
						</NavItem>
						<NavItem asChild className="hidden md:inline-block">
							<a
								href="https://rivet.gg/docs"
								target="_blank"
								rel="noreferrer"
							>
								Docs
							</a>
						</NavItem>
						<UserProfileButton />
					</div>
				</div>
				<HeaderSubNav />
			</div>
		</header>
	);
}
