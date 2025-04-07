import { connectionStateAtom } from "@/stores/manager";
import { cn, DocsSheet, ShimmerLine } from "@rivet-gg/components";
import { Header as RivetHeader, NavItem } from "@rivet-gg/components/header";
import { faGithub, Icon } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { useAtomValue } from "jotai";
import type { PropsWithChildren, ReactNode } from "react";

interface RootProps {
	children: ReactNode;
}

const Root = ({ children }: RootProps) => {
	return <div className={cn("flex min-h-screen flex-col")}>{children}</div>;
};

const Main = ({ children }: RootProps) => {
	return (
		<main className="bg-background flex flex-1 flex-col h-full min-h-0 relative">
			{children}
		</main>
	);
};

const VisibleInFull = ({ children }: PropsWithChildren) => {
	return (
		<div className="relative min-h-screen max-h-screen grid grid-rows-[auto,1fr]">
			{children}
		</div>
	);
};

const Header = () => {
	const connectionStatus = useAtomValue(connectionStateAtom);
	return (
		<RivetHeader
			logo={<img src="/logo.svg" alt="Rivet.gg" className="h-6" />}
			addons={
				connectionStatus !== "connected" ? (
					<ShimmerLine className="-bottom-1" />
				) : null
			}
			links={
				<>
					<NavItem asChild>
						<a href="https://github.com/rivet-gg/rivet">
							<Icon icon={faGithub} />
						</a>
					</NavItem>
					<DocsSheet
						path={"https://actorcore.org/overview"}
						title="Documentation"
					>
						<NavItem className="cursor-pointer">
							Documentation
						</NavItem>
					</DocsSheet>
					<NavItem asChild>
						<Link
							to="."
							search={(old) => ({ ...old, modal: "feedback" })}
						>
							Feedback
						</Link>
					</NavItem>
				</>
			}
		/>
	);
};

const Footer = () => {
	return null;
};

export { Root, Main, Header, Footer, VisibleInFull };
