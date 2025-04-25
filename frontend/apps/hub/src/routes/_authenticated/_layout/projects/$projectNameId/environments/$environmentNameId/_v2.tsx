import { Button, cn, WithTooltip } from "@rivet-gg/components";
import { ActorsLayout, useActorsLayout } from "@rivet-gg/components/actors";
import {
	faActorsBorderless,
	faBarsStaggered,
	faCodeBranch,
	faCog,
	faFunction,
	faServer,
	faSidebar,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { createFileRoute, Link, Outlet } from "@tanstack/react-router";
import { AnimatePresence, motion } from "framer-motion";

const SIDEBAR: (
	| { type: "separator" }
	| { icon: IconProp; label: string; to: string; isDisabled?: boolean }
)[] = [
	{ icon: faActorsBorderless, label: "Actors", to: "actors" },
	{ icon: faServer, label: "Containers", to: "containers" },
	{ icon: faFunction, label: "Functions", to: "functions" },
	{ icon: faBarsStaggered, label: "Logs", to: "logs" },
	{ icon: faCodeBranch, label: "Versions", to: "actor-versions" },
	{ type: "separator" },
	{ icon: faCog, label: "Settings", to: "settings" },
];

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/_v2",
)({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<ActorsLayout
			className="h-full min-h-full max-h-full w-full min-w-full max-w-full"
			left={<Sidebar />}
			right={
				<div className="h-full overflow-auto">
					<Outlet />
				</div>
			}
		/>
	);
}

function Sidebar() {
	const { isFolded, setFolded } = useActorsLayout();
	return (
		<motion.div
			className="p-4 flex flex-col justify-between border-r"
			animate={{
				width: isFolded ? 74 : 192,
				minWidth: isFolded ? "74px" : "192px",
			}}
		>
			<ul className="w-full flex flex-col gap-1">
				{SIDEBAR.map((item, index) => {
					if ("type" in item) {
						return (
							<li key={index}>
								<hr className="my-2" />
							</li>
						);
					}

					const button = ({ isActive }: { isActive: boolean }) => (
						<Button
							disabled={item.isDisabled}
							className={cn(
								"transition-all min-w-12 text-left justify-start",
								{
									"w-full": !isFolded,
									"text-foreground": isActive,
									"bg-secondary": isActive,
									"text-muted-foreground": !isActive,
								},
							)}
							variant="ghost"
							startIcon={<Icon icon={item.icon} />}
						>
							<span>
								<AnimatePresence>
									{isFolded ? null : (
										<motion.span
											key="label"
											initial={{
												opacity: 0,
												scale: 0.95,
											}}
											animate={{
												opacity: 1,
												scale: 1,
											}}
											exit={{
												opacity: 0,
												scale: 0.95,
											}}
											className="flex-1 min-w-0"
										>
											{item.label}
										</motion.span>
									)}
								</AnimatePresence>
							</span>
						</Button>
					);

					if (item.isDisabled) {
						return (
							<li key={index} className="w-full">
								<WithTooltip
									trigger={
										<div>{button({ isActive: false })}</div>
									}
									content="Coming soon"
								/>
							</li>
						);
					}

					return (
						<li key={index} className="w-full">
							<Link
								to={item.to}
								from="/projects/$projectNameId/environments/$environmentNameId/"
							>
								{({ isActive }) => {
									return button({ isActive });
								}}
							</Link>
						</li>
					);
				})}
			</ul>
			<Button
				size="icon"
				variant="ghost"
				onClick={() => setFolded(!isFolded)}
			>
				<Icon icon={faSidebar} />
			</Button>
		</motion.div>
	);
}
