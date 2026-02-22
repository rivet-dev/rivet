"use client";
import { usePathname } from "@/hooks/usePathname";
import { Popover, Transition } from "@headlessui/react";
import clsx from "clsx";
import { motion, useScroll, useTransform } from "framer-motion";
import { forwardRef } from "react";
import { Fragment } from "react";

import { Button } from "@/components/Button";
import {
	MobileNavigation,
	useIsInsideMobileNavigation,
} from "@/components/MobileNavigation";
import { useNavigation } from "@/hooks/useNavigation";
import imgLogo from "@/images/rivet-logos/icon-cream.svg";
import imgLogoText from "@/images/rivet-logos/icon-text-cream.svg";
import {
	Icon,
	faCircle,
	faDiamond,
	faDiscord,
	faFlask,
	faGithub,
	faHexagon,
	faRhombus,
	faSquare,
} from "@rivet-gg/icons";

const ICONS = {
	godot: faHexagon,
	unity: faSquare,
	unreal: faCircle,
	html5: faRhombus,
	custom: faDiamond,
};

function TopLevelNavItem({ href, target, initHref, icon, children }) {
	const pathname = usePathname();

	const current = pathname?.startsWith(href);
	return (
		<a href={initHref ?? href}
			target={target}
			className={clsx(
				current
					? "border-white/10 bg-white/5 text-white"
					: "text-cream-100 hover:bg-white/5 hover:text-white",
				"border-1 flex items-center gap-2.5 border border-transparent px-3.5 py-1.5 transition",
			)}
		>
			{icon ? <Icon icon={icon} /> : null}
			<span className="font-display text-lg">{children}</span>
		</a>
	);
}

function TopLevelNavPopover({ solutions, callsToAction, children }) {
	return (
		<Popover className="relative">
			<Popover.Button className="-mr-1 inline-flex items-center gap-x-1 text-sm leading-5 text-cream-400  transition hover:text-white">
				<span>{children}</span>
				{/* <ChevronDownIcon className='h-5 w-5' aria-hidden='true' /> */}
			</Popover.Button>

			<Transition
				as={Fragment}
				enter="transition ease-out duration-200"
				enterFrom="opacity-0 translate-y-1"
				enterTo="opacity-100 translate-y-0"
				leave="transition ease-in duration-150"
				leaveFrom="opacity-100 translate-y-0"
				leaveTo="opacity-0 translate-y-1"
			>
				<Popover.Panel className="absolute z-10 mt-5 flex w-screen max-w-max">
					<div className="w-screen max-w-md flex-auto overflow-hidden rounded-3xl bg-white text-sm leading-6 shadow-lg ring-1 ring-gray-900/5">
						<div className="p-4">{solutions}</div>
						<div className="grid grid-cols-2 divide-x divide-gray-900/5 bg-gray-50">
							{callsToAction}
						</div>
					</div>
				</Popover.Panel>
			</Transition>
		</Popover>
	);
}

function TopLevelNavPopoverSolution({ icon, href, title, description }) {
	return (
		<div
			key={title}
			className="group relative flex gap-x-6 rounded-lg p-4 hover:bg-gray-50"
		>
			<div className="mt-1 flex h-11 w-11 flex-none items-center justify-center rounded-lg bg-gray-50 group-hover:bg-white">
				<Icon
					icon={icon}
					className="h-6 w-6 text-gray-600 group-hover:text-violet-600"
					aria-hidden="true"
				/>
			</div>
			<div>
				<a href={href} className="font-semibold text-gray-900">
					{title}
					<span className="absolute inset-0" />
				</a>
				<p className="mt-1 text-gray-600">{description}</p>
			</div>
		</div>
	);
}

function TopLevelNavPopoverCallToAction({ icon, href, title }) {
	return (
		<a
			key={title}
			href={href}
			className="flex items-center justify-center gap-x-2.5 p-3 font-semibold text-gray-900 hover:bg-gray-100"
		>
			<Icon
				icon={icon}
				className="h-5 w-5 flex-none text-gray-400"
				aria-hidden="true"
			/>
			{title}
		</a>
	);
}

const StatusBadge = ({ status }) => {
	if (status === "experimental") {
		return (
			<span className="text-2xs text-amber-500">
				<Icon icon={faFlask} />
			</span>
		);
	}
};

export const Header = forwardRef(function Header(
	{ className, tabsTitle, tabs },
	ref,
) {
	const { navigation } = useNavigation();
	const pathname = usePathname();
	const isInsideMobileNavigation = useIsInsideMobileNavigation();

	const { scrollY } = useScroll();
	const bgOpacityLight = useTransform(scrollY, [0, 72], [0.5, 0.9]);
	const bgOpacityDark = useTransform(scrollY, [0, 72], [0.2, 0.8]);

	return (
		<>
			<motion.div
				ref={ref}
				className={clsx(
					className,
					"pointer-events-auto z-50 flex w-full flex-col transition",
					!isInsideMobileNavigation && "backdrop-blur",
					isInsideMobileNavigation
						? "bg-charcole-950"
						: "bg-charcole-950/[var(--bg-opacity-dark)]",
				)}
				style={{
					"--bg-opacity-light": bgOpacityLight,
					"--bg-opacity-dark": bgOpacityDark,
				}}
			>
				{/* Main header */}
				<div className="main-content-container flex h-14 w-full items-center justify-between gap-12 px-6 lg:z-30">
					<div className="flex items-center gap-4">
						<div className="xl:hidden">
							<MobileNavigation navigation={navigation} />
						</div>

						<a href="/" aria-label="Home" className="xl:hidden">
							<img src={imgLogo.src}
								alt="Rivet"
								className="h-6 w-auto"
							/>
						</a>

						<a href="/"
							aria-label="Home"
							className="hidden xl:block"
						>
							<img src={imgLogoText.src}
								alt="Rivet"
								className="h-6 w-auto"
							/>
						</a>

						<div className="hidden items-center gap-1 lg:flex">
							<Popover className="relative">
								<Popover.Button
									className={clsx(
										"text-cream-100 hover:bg-white/5 hover:text-white",
										"border-1 flex items-center gap-2.5 border border-transparent px-3.5 py-1.5 transition",
									)}
								>
									<span className="font-display text-lg">Products</span>
								</Popover.Button>

								<Transition
									as={Fragment}
									enter="transition ease-out duration-200"
									enterFrom="opacity-0 translate-y-1"
									enterTo="opacity-100 translate-y-0"
									leave="transition ease-in duration-150"
									leaveFrom="opacity-100 translate-y-0"
									leaveTo="opacity-0 translate-y-1"
								>
									<Popover.Panel className="absolute left-0 z-10 mt-2 w-56 rounded-lg bg-charcole-900 border border-white/10 shadow-lg">
										<div className="py-2">
											<a
												href="/docs/actors"
												className="block px-4 py-2 text-sm text-cream-100 hover:bg-white/5 hover:text-white transition"
											>
												Actors
											</a>
											<a
												href="https://sandboxagent.dev"
												target="_blank"
												rel="noopener noreferrer"
												className="block px-4 py-2 text-sm text-cream-100 hover:bg-white/5 hover:text-white transition"
											>
												Sandbox Agent SDK
											</a>
										</div>
									</Popover.Panel>
								</Transition>
							</Popover>
							<TopLevelNavItem href="/docs">Documentation</TopLevelNavItem>
							<TopLevelNavItem href="/changelog">
								Changelog
							</TopLevelNavItem>
							<TopLevelNavItem href="/cloud">
								Cloud
							</TopLevelNavItem>
						</div>
					</div>

					<div className="flex items-center gap-4">
						{/* Social icons */}
						{/* <div className='flex gap-1.5'> */}
						{[
							[faDiscord, "https://rivet.dev/discord"],
							[faGithub, "https://github.com/rivet-dev/rivet"],
						].map(([icon, href]) => (
							<a
								className="flex items-center justify-center p-1 opacity-75 transition hover:opacity-100"
								key={href}
								href={href}
								target="_blank"
							>
								<Icon
									icon={icon}
									className="text-lg text-cream-100"
								/>
							</a>
						))}
						{/* </div> */}

						{/* <ModeToggle /> */}
						<div className="hidden min-[416px]:contents">
							<Button
								href="https://dashboard.rivet.dev"
								variant="secondary"
							>
								Sign In
							</Button>
						</div>
					</div>
				</div>

				{/* Tabs */}
				{(navigation?.tabs || tabs) && (
					<div
						className={
							"hide-scrollbar main-content-container h-12 w-full overflow-x-scroll px-6"
						}
					>
						{/* Border */}
						<div className="absolute inset-x-0 bottom-0 h-[2px] bg-cream-100/5"></div>

						<nav className="relative flex h-full space-x-4">
							{/* Title */}
							<div className="text-md flex items-center font-display font-bold text-charcole-900 text-white">
								{navigation.tabsTitle || tabsTitle}
							</div>

							{/* Tabs */}
							{(navigation.tabs || tabs).map((tab) => (
								<a
									key={tab.title}
									href={tab.href}
									className={clsx(
										"flex h-full shrink-0 items-center gap-1 whitespace-nowrap border-b-2 px-1 pt-1 text-sm font-semibold transition",
										tab.current ||
											pathname.startsWith(tab.href)
											? "border-charcole-900 border-cream-50"
											: "border-transparent opacity-80 hover:border-charcole-900 hover:border-white hover:opacity-100",
										tab.styles?.text ?? "text-white",
									)}
									aria-current={
										pathname.startsWith(tab.href)
											? "page"
											: undefined
									}
								>
									{tab.icon ? (
										<Icon
											icon={ICONS[tab.icon]}
											className="mx-1 h-3.5 w-3.5"
											alt="Tab icon"
										/>
									) : null}
									<span>{tab.title}</span>
									<StatusBadge status={tab.status} />
								</a>
							))}
						</nav>
					</div>
				)}
			</motion.div>
		</>
	);
});
