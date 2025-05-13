import { CommandPanel } from "@/components/command-panel";
import { Button, cn } from "@rivet-gg/components";
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

const SIDEBAR: {
	label: string;
	items: {
		icon: IconProp;
		label: string;
		to: string;
		isDisabled?: boolean;
	}[];
}[] = [
	{
		label: "products",
		items: [
			{ icon: faActorsBorderless, label: "Actors", to: "actors" },
			{ icon: faServer, label: "Containers", to: "containers" },
			{ icon: faFunction, label: "Functions", to: "functions" },
		],
	},
	{
		label: "tools",
		items: [
			{ icon: faBarsStaggered, label: "Logs", to: "logs" },
			{ icon: faCodeBranch, label: "Versions", to: "actor-versions" },
		],
	},
	{
		label: "settings",
		items: [{ icon: faCog, label: "Settings", to: "settings" }],
	},
];

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/_v2",
)({
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<>
			<ActorsLayout
				className="h-full min-h-full max-h-full w-full min-w-full max-w-full"
				left={<Sidebar />}
				right={
					<div className="h-full overflow-auto">
						<Outlet />
					</div>
				}
			/>
		</>
	);
}

function Sidebar() {
	const { isFolded, setFolded } = useActorsLayout();
	return (
		<motion.div
			className="p-2 flex flex-col justify-between border-r"
			animate={{
				width: isFolded ? 74 : 192,
				minWidth: isFolded ? "74px" : "192px",
			}}
		>
			<ul className="w-full flex flex-col gap-2">
				{SIDEBAR.map((item, index) => {
					if ("items" in item) {
						return (
							<li key={index} className="w-full">
								<ul className="flex flex-col gap-1">
									{item.items.map((subItem, subIndex) => {
										return (
											<li
												key={subIndex}
												className="w-full"
											>
												<Link
													to={subItem.to}
													from="/projects/$projectNameId/environments/$environmentNameId/"
												>
													{({ isActive }) => {
														return (
															<SidebarItem
																isActive={
																	isActive
																}
																label={
																	subItem.label
																}
																icon={
																	subItem.icon
																}
															/>
														);
													}}
												</Link>
											</li>
										);
									})}
								</ul>

								{index !== SIDEBAR.length - 1 && (
									<hr className="mt-2" />
								)}
							</li>
						);
					}
				})}
			</ul>
			<div className="flex flex-col gap-2">
				<CommandPanel className="lg:w-full md:w-full" />
				<Button
					size="icon"
					variant="ghost"
					onClick={() => setFolded(!isFolded)}
				>
					<Icon icon={faSidebar} />
				</Button>
			</div>
		</motion.div>
	);
}

function SidebarItem({
	isActive,
	label,
	icon,
}: { isActive: boolean; label: string; icon: IconProp }) {
	const { isFolded, setFolded } = useActorsLayout();
	return (
		<Button
			className={cn(
				"transition-all min-w-12 text-left justify-start px-2 py-2 h-auto pl-6",
				{
					"w-full": !isFolded,
					"text-foreground": isActive,
					"bg-secondary": isActive,
					"text-muted-foreground": !isActive,
				},
			)}
			variant="ghost"
			startIcon={<Icon icon={icon} />}
		>
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
						{label}
					</motion.span>
				)}
			</AnimatePresence>
		</Button>
	);
}
