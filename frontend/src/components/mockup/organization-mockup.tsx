import {
	Avatar,
	AvatarFallback,
	AvatarImage,
	Button,
} from "@/components";

function SettingsCard({
	title,
	description,
	action,
	children,
}: {
	title: string;
	description?: string;
	action?: React.ReactNode;
	children?: React.ReactNode;
}) {
	return (
		<div className="rounded-xl border dark:border-white/10 bg-card overflow-hidden">
			<div className="flex items-start justify-between gap-4 px-6 pt-5 pb-4">
				<div>
					<h3 className="text-base font-semibold text-foreground">
						{title}
					</h3>
					{description ? (
						<p className="mt-0.5 text-xs text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{action}
			</div>
			{children}
		</div>
	);
}

export function OrganizationContent() {
	return (
		<div className="space-y-6 pb-10">
			<SettingsCard
				title="Organization profile"
				description="The avatar and display name shown to your teammates."
				action={
					<Button
						variant="outline"
						size="sm"
						className="shrink-0"
					>
						Update profile
					</Button>
				}
			>
				<div className="flex items-center gap-3 border-t dark:border-white/10 px-6 py-4">
					<Avatar className="size-10 rounded-md">
						<AvatarImage
							src="https://avatar.vercel.sh/test-projects"
							alt="test projects"
						/>
						<AvatarFallback className="rounded-md text-sm font-medium">
							T
						</AvatarFallback>
					</Avatar>
					<div className="flex flex-col min-w-0">
						<span className="text-sm font-medium text-foreground truncate">
							test projects
						</span>
						<span className="text-xs text-muted-foreground truncate">
							test-projects
						</span>
					</div>
				</div>
			</SettingsCard>

			<SettingsCard
				title="Leave organization"
				description="Revoke your access to this organization. You will lose access to all of its projects."
				action={
					<Button
						variant="outline"
						size="sm"
						className="shrink-0"
					>
						Leave organization
					</Button>
				}
			/>

			<section className="rounded-xl border border-destructive/30 bg-destructive/5 px-6 py-5">
				<div className="flex items-start justify-between gap-4">
					<div className="min-w-0">
						<h4 className="text-sm font-semibold text-destructive">
							Delete organization
						</h4>
						<p className="mt-1 text-xs text-muted-foreground leading-relaxed">
							Permanently delete this organization and all of its
							projects. This action cannot be undone.
						</p>
					</div>
					<Button
						variant="destructive"
						size="sm"
						className="shrink-0"
					>
						Delete organization
					</Button>
				</div>
			</section>
		</div>
	);
}
