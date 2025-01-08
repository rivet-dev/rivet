"use client";
import { faBellConcierge, faHammer, faPlus } from "@rivet-gg/icons";
import { Icon } from "@rivet-gg/icons";
import { LayoutGroup, motion } from "framer-motion";
import { useState } from "react";
import type { Category } from "./lib/modules";
import { cn } from "./lib/utils";
import { DocumentedModuleCard, ModuleCard } from "./module-card";
import { SidebarNavigation } from "./sidebar-navigation";
import { SidebarPage } from "./sidebar-page";
import { Button } from "./ui/button";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
} from "./ui/card";
import { Flex } from "./ui/flex";
import { Grid } from "./ui/grid";
import { Input } from "./ui/input";
import { H1, H2, Lead } from "./ui/typography";

interface ModulesStoreProps {
	categories: Category[];
	onModuleClick?: (module: Category["modules"][number]["module"]) => void;
	includeModulesDocumentation?: boolean;
}

export function ModulesStore({
	categories,
	includeModulesDocumentation = false,
	onModuleClick,
}: ModulesStoreProps) {
	const [query, setQuery] = useState("");

	const filteredCategories = categories
		.map((category) => {
			const modules = category.modules.filter(
				({ module }) =>
					module.config.name
						.toLowerCase()
						.includes((query || "").toLowerCase()) ||
					module.config.description
						.toLowerCase()
						.includes((query || "").toLowerCase()),
			);
			return { ...category, modules };
		})
		.filter((category) => category.modules.length > 0);

	return (
		<>
			<section className="flex flex-col gap-4 my-8">
				<H1 className="text-center">Backend Modules</H1>
				<Lead className="text-center">
					Build your game&apos;s backend with open-source modules.
				</Lead>

				<Input asChild className="max-w-lg mx-auto">
					<Flex items="center">
						<Icon
							icon="magnifying-glass"
							className="text-muted-foreground mr-2"
						/>
						<input
							value={query}
							onChange={(e) => {
								setQuery(e.target.value);
							}}
							placeholder="Search..."
							className="bg-transparent border-transparent h-full w-full placeholder:text-muted-foreground focus-visible:outline-none"
						/>
					</Flex>
				</Input>
			</section>
			<SidebarPage
				sidebar={
					<SidebarNavigation>
						{categories.map((category) => (
							<a
								key={category.slug}
								href={`#${category.slug}`}
								className={cn(
									"transition-opacity",
									filteredCategories.find(
										(c) => c.slug === category.slug,
									)
										? "text-foreground"
										: "opacity-50",
								)}
							>
								{category.name}
							</a>
						))}

						<Button
							variant="outline"
							asChild
							startIcon={<Icon icon={faPlus} />}
						>
							<a
								href="https://rivet.gg/docs/general/modules/build/overview"
								target="_blank"
								rel="noopener noreferrer"
							>
								Build Your Own
							</a>
						</Button>
					</SidebarNavigation>
				}
			>
				<div>
					<LayoutGroup>
						{filteredCategories.length === 0 ? (
							<Card className="w-full lg:w-1/2" asChild>
								<motion.div
									initial={{ opacity: 0 }}
									animate={{ opacity: 1 }}
									exit={{ opacity: 0 }}
								>
									<CardHeader className="text-left">
										<CardTitle>
											<Icon
												icon="sad-tear"
												className="text-2xl mr-2"
											/>
											No modules found
										</CardTitle>
									</CardHeader>
									<CardContent>
										If you can&apos;t find a module that
										fits your needs, you can request a
										module to be created or build your own
										module.
									</CardContent>
									<CardFooter className="gap-2">
										<Button
											asChild
											startIcon={
												<Icon icon={faBellConcierge} />
											}
										>
											<a
												href="https://b8v8449klvp.typeform.com/to/kpcSBpuP"
												target="_blank"
												rel="noopener noreferrer"
											>
												Request Module
											</a>
										</Button>
										<Button
											asChild
											startIcon={<Icon icon={faHammer} />}
										>
											<a
												href="https://rivet.gg/docs/general/modules/build/overview"
												target="_blank"
												rel="noopener noreferrer"
											>
												Build Your Own Module
											</a>
										</Button>
									</CardFooter>
								</motion.div>
							</Card>
						) : null}
						{filteredCategories.map((category) => (
							<motion.section
								layout="position"
								layoutId={category.slug}
								key={category.slug}
								className="mb-10"
								initial={{ opacity: 0 }}
								animate={{ opacity: 1 }}
								exit={{ opacity: 0 }}
							>
								<H2 id={category.slug}>{category.name}</H2>
								<p className="text-muted-foreground mb-6 mt-2">
									{category.description}
								</p>
								<Grid
									columns={{ initial: "1", sm: "3" }}
									gap="4"
									items="start"
								>
									{category.modules.map(({ id, module }) =>
										includeModulesDocumentation ? (
											<DocumentedModuleCard
												key={id}
												id={id}
												{...module.config}
											/>
										) : (
											<ModuleCard
												key={id}
												id={id}
												{...module.config}
												onClick={() =>
													onModuleClick?.(module)
												}
											/>
										),
									)}
								</Grid>
							</motion.section>
						))}
					</LayoutGroup>
				</div>
			</SidebarPage>
		</>
	);
}
