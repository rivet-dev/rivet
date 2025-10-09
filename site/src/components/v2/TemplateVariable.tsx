"use client";

import {
	Button,
	Popover,
	PopoverContent,
	PopoverTrigger,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@rivet-gg/components";
import { faRightToBracket, Icon } from "@rivet-gg/icons";
import { useEffect, useMemo, useState } from "react";
import { useAutofillStore } from "@/stores/autofill-store";

interface TemplateVariableProps {
	variable: string;
	defaultValue?: string;
}

const CLOUD_LOGIN_URL =
	process.env.NEXT_PUBLIC_CLOUD_URL || "https://dashboard.rivet.dev";

export function TemplateVariable({ variable, defaultValue }: TemplateVariableProps) {
	const [open, setOpen] = useState(false);
	const {
		user,
		projects,
		namespaces,
		selectedProjectId,
		selectedNamespaceId,
		isLoading,
		isLoadingNamespaces,
		initialize,
		selectProject,
		selectNamespace,
		getTemplateVariables,
	} = useAutofillStore();

	useEffect(() => {
		initialize();
	}, [initialize]);

	const selectedProject = useMemo(
		() => projects.find((p) => p.id === selectedProjectId),
		[projects, selectedProjectId],
	);

	const selectedNamespace = useMemo(
		() => namespaces.find((n) => n.id === selectedNamespaceId),
		[namespaces, selectedNamespaceId],
	);

	const value = useMemo(() => {
		const variables = getTemplateVariables();
		// Use actual value if available, fallback to default, then template placeholder
		return variables[variable] || defaultValue || `{{${variable}}}`;
	}, [variable, defaultValue, getTemplateVariables]);

	// Not authenticated
	if (!isLoading && !user) {
		return (
			<Popover open={open} onOpenChange={setOpen}>
				<PopoverTrigger asChild>
					<button
						type="button"
						className="underline decoration-dotted underline-offset-2 hover:bg-yellow-400/20 cursor-pointer rounded px-0.5"
					>
						{defaultValue || `{{${variable}}}`}
					</button>
				</PopoverTrigger>
				<PopoverContent className="w-80 bg-card">
					<div className="space-y-3">
						<div>
							<h4 className="font-medium">Sign in required</h4>
							<p className="text-muted-foreground text-sm mt-1">
								Connect your Rivet account to autofill this
								value
							</p>
						</div>
						<Button asChild className="w-full">
							<a href={CLOUD_LOGIN_URL}>
								<Icon icon={faRightToBracket} />
								Sign In
							</a>
						</Button>
					</div>
				</PopoverContent>
			</Popover>
		);
	}

	// Loading
	if (isLoading) {
		return (
			<span className="opacity-50 cursor-wait">{defaultValue || `{{${variable}}}`}</span>
		);
	}

	// No projects
	if (projects.length === 0) {
		return (
			<Popover open={open} onOpenChange={setOpen}>
				<PopoverTrigger asChild>
					<button
						type="button"
						className="underline decoration-dotted underline-offset-2 hover:bg-yellow-400/20 cursor-pointer rounded px-0.5"
					>
						{defaultValue || `{{${variable}}}`}
					</button>
				</PopoverTrigger>
				<PopoverContent className="w-80">
					<div className="space-y-2">
						<h4 className="font-medium">No projects found</h4>
						<p className="text-muted-foreground text-sm">
							Create a project on Rivet Cloud to use autofill
						</p>
					</div>
				</PopoverContent>
			</Popover>
		);
	}

	// Has value - show it with click to change
	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<button
					type="button"
					className="bg-blue-500/10 text-blue-600 dark:text-blue-400 rounded px-1 py-0.5 hover:bg-blue-500/20 cursor-pointer font-semibold"
				>
					{value || `{{${variable}}}`}
				</button>
			</PopoverTrigger>
			<PopoverContent className="w-96">
				<div className="space-y-4">
					<div>
						<h4 className="font-medium mb-1">Configure autofill</h4>
						<p className="text-muted-foreground text-xs">
							Select your project and namespace
						</p>
					</div>

					{/* Project Selector */}
					<div className="space-y-2">
						<label className="text-sm font-medium">Project</label>
						<Select
							value={selectedProjectId || undefined}
							onValueChange={selectProject}
						>
							<SelectTrigger className="w-full">
								<SelectValue placeholder="Select a project">
									{selectedProject
										? `${selectedProject.organizationSlug} / ${selectedProject.slug}`
										: "Select a project"}
								</SelectValue>
							</SelectTrigger>
							<SelectContent>
								{projects.map((project) => (
									<SelectItem
										key={project.id}
										value={project.id}
									>
										{project.organizationSlug} /{" "}
										{project.slug}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>

					{/* Namespace Selector */}
					<div className="space-y-2">
						<label className="text-sm font-medium">Namespace</label>
						<Select
							value={selectedNamespaceId || undefined}
							onValueChange={selectNamespace}
							disabled={!selectedProjectId || isLoadingNamespaces}
						>
							<SelectTrigger className="w-full">
								<SelectValue placeholder="Select a namespace">
									{isLoadingNamespaces
										? "Loading..."
										: selectedNamespace
											? selectedNamespace.slug
											: "Select a namespace"}
								</SelectValue>
							</SelectTrigger>
							<SelectContent>
								{namespaces.map((namespace) => (
									<SelectItem
										key={namespace.id}
										value={namespace.id}
									>
										{namespace.slug}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>
				</div>
			</PopoverContent>
		</Popover>
	);
}
