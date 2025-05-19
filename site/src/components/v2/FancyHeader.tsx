"use client";
import { DocsMobileNavigation } from "@/components/DocsMobileNavigation";
import { GitHubStars } from "@/components/GitHubStars";
import logoUrl from "@/images/rivet-logos/icon-text-white.svg";
import { Button, cn } from "@rivet-gg/components";
import { Header as RivetHeader } from "@rivet-gg/components/header";
import { Icon, faDiscord } from "@rivet-gg/icons";
import Image from "next/image";
import Link from "next/link";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { HeaderPopupProductMenu } from "../HeaderPopupProductMenu";

interface TextNavItemProps {
  href: string;
  children: ReactNode;
  onMouseEnter?: () => void;
  className?: string;
  ariaCurrent?: boolean | "page" | "step" | "location" | "date" | "time";
}

function TextNavItem({ href, children, onMouseEnter, className, ariaCurrent }: TextNavItemProps) {
  return (
    <div className={cn("px-2.5 py-2 opacity-60 hover:opacity-100 transition-all duration-200", className)}>
      <RivetHeader.NavItem
        asChild
        onMouseEnter={onMouseEnter}
      >
        <Link
          href={href}
          className="text-white"
          aria-current={ariaCurrent}
        >
          {children}
        </Link>
      </RivetHeader.NavItem>
    </div>
  );
}

type Subnav = false | "product" | "solutions";

interface FancyHeaderProps {
	active?: "product" | "docs" | "blog" | "pricing";
	subnav?: ReactNode;
	mobileBreadcrumbs?: ReactNode;
}

export function FancyHeader({
	active,
	subnav,
	mobileBreadcrumbs,
}: FancyHeaderProps) {
	const [isSubnavOpen, setIsSubnavOpen] = useState<Subnav>(false);
	const prev = useRef<Subnav>(false);

	useEffect(() => {
		prev.current = isSubnavOpen;
	}, [isSubnavOpen]);

	const headerStyles = cn(
		"md:border-transparent md:static md:bg-transparent md:rounded-2xl md:max-w-[1200px] md:border-transparent md:backdrop-none [&>div:first-child]:px-3 md:backdrop-blur-none",
	);
	return (
		<>
			<div
				className={cn(
					"pointer-events-none fixed inset-0 z-50 hidden backdrop-blur-sm transition-opacity md:block",
					isSubnavOpen ? "opacity-100" : "opacity-0",
				)}
			/>
			<motion.div
				className="fixed top-0 z-50  w-full max-w-[1200px] md:left-1/2 md:top-4 md:-translate-x-1/2 md:px-8"
				onMouseLeave={() => setIsSubnavOpen(false)}
			>
				<motion.div className='relative before:pointer-events-none before:absolute  before:inset-[-1px] before:z-20  before:hidden before:rounded-2xl before:border before:border-white/10 before:content-[""] md:before:block'>
					<motion.div
						className={cn(
							"absolute inset-0 -z-[1] hidden overflow-hidden rounded-2xl transition-all md:block",
							isSubnavOpen
								? "bg-background backdrop-blur-0 backdrop-saturate-0"
								: "bg-background/80 backdrop-blur-lg",
						)}
					/>
					<RivetHeader
						className={headerStyles}
						logo={
							<Link href="/">
								<Image
									src={logoUrl.src || logoUrl}
									width={80}
									height={24}
									className="ml-1 w-20"
									alt="Rivet logo"
									unoptimized
								/>
							</Link>
						}
						subnav={subnav}
						support={
							<div className="flex flex-col gap-4 font-v2 subpixel-antialiased">
								<RivetHeader.NavItem asChild>
									<Link href="https://hub.rivet.gg">
										Sign In
									</Link>
								</RivetHeader.NavItem>
								<RivetHeader.NavItem asChild>
									<Link href="/discord">Discord</Link>
								</RivetHeader.NavItem>
								<RivetHeader.NavItem asChild>
									<Link href="/support">Support</Link>
								</RivetHeader.NavItem>
							</div>
						}
						links={
							<div className="flex flex-row items-center">
								<RivetHeader.NavItem
									asChild
									className="p-2 mr-4"
								>
									<Link
										href="/discord"
										className="text-white/90"
									>
										<Icon
											icon={faDiscord}
											className="drop-shadow-md"
										/>
									</Link>
								</RivetHeader.NavItem>
								<GitHubStars 
									className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 h-10 text-sm mr-2 hover:border-white/20 text-white/90 hover:text-white transition-colors" 
								/>
								<Link 
									href="https://hub.rivet.gg" 
									className="font-v2 subpixel-antialiased inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white shadow-sm hover:border-white/20 transition-colors"
								>
									Sign In
								</Link>
							</div>
						}
						mobileBreadcrumbs={<DocsMobileNavigation />}
						breadcrumbs={
							<div className="flex items-center font-v2 subpixel-antialiased">
								{/*<RivetHeader.NavItem
									asChild
									className="flex cursor-pointer items-center gap-1 px-2.5 py-2 first:pl-0 "
									onMouseEnter={() =>
										setIsSubnavOpen("product")
									}
								>
									<div
										className="text-white/90"
										aria-current={
											active === "product"
												? "page"
												: undefined
										}
									>
										Product
									</div>
								</RivetHeader.NavItem>*/}
								{/* <RivetHeader.NavItem
                  asChild
                  className='flex cursor-pointer items-center gap-1 px-2.5 py-2'
                  onMouseEnter={() => setIsSubnavOpen('solutions')}>
                  <div className='text-white/90'>Solutions</div>
                </RivetHeader.NavItem> */}
								<TextNavItem 
									href="/docs"
									onMouseEnter={() => setIsSubnavOpen(false)}
									ariaCurrent={active === "docs" ? "page" : undefined}
								>
									Documentation
								</TextNavItem>
								<TextNavItem 
									href="/changelog"
									onMouseEnter={() => setIsSubnavOpen(false)}
									ariaCurrent={active === "blog" ? "page" : undefined}
								>
									Changelog
								</TextNavItem>
								<TextNavItem 
									href="/pricing"
									onMouseEnter={() => setIsSubnavOpen(false)}
									ariaCurrent={active === "pricing" ? "page" : undefined}
								>
									Pricing
								</TextNavItem>
							</div>
						}
					/>
					<AnimatePresence>
						{isSubnavOpen ? (
							<motion.div
								className="relative overflow-hidden"
								initial={{ height: 0, opacity: 1 }}
								animate={{
									height: 200,
									opacity: 1,
									transition: { ease: ["easeIn", "easeOut"] },
								}}
								exit={{ height: 0, opacity: 0 }}
							>
								<AnimatePresence>
									{isSubnavOpen === "product" ? (
										<motion.div
											key="product"
											onMouseLeave={() =>
												setIsSubnavOpen(false)
											}
											className=" absolute inset-0"
										>
											<motion.div
												initial={{
													opacity: 0,
													y:
														prev.current ===
														"solutions"
															? -10
															: 0,
												}}
												animate={{ opacity: 1, y: 0 }}
												exit={{ opacity: 0, y: 0 }}
												className="overflow-hidden"
											>
												<div className="h-[200px] w-full overflow-hidden pb-4 pl-4 pr-4 pt-2">
													<HeaderPopupProductMenu />
												</div>
											</motion.div>
										</motion.div>
									) : null}
									{/* {isSubnavOpen === 'solutions' ? (
                    <motion.div
                      key='solutions'
                      onMouseLeave={() => setIsSubnavOpen(false)}
                      className='absolute inset-0'>
                      <motion.div
                        initial={{ opacity: 0, y: prev.current === 'product' ? -10 : 0 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: 0 }}
                        className='overflow-hidden'>
                        <div className='h-[200px] w-full overflow-hidden pb-4 pl-4 pr-4 pt-2'>
                          <HeaderPopupSolutionsMenu />
                        </div>
                      </motion.div>
                    </motion.div>
                  ) : null} */}
								</AnimatePresence>
							</motion.div>
						) : null}
					</AnimatePresence>
				</motion.div>
			</motion.div>
		</>
	);
}
