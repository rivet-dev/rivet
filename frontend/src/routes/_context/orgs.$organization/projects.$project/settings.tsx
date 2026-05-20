import { faQuestionCircle, faTrash, faTriangleExclamation, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute, notFound, useNavigate } from "@tanstack/react-router";
import { HelpDropdown } from "@/app/help-dropdown";
import { Content } from "@/app/layout";
import { RouteLayout } from "@/app/route-layout";
import { Button, H1, SmallText } from "@/components";
import { useCloudProjectDataProvider } from "@/components/actors";
import { features } from "@/lib/features";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/settings",
)({
	beforeLoad: () => {
		if(!features.platform) {
			throw notFound();
		}
	
	},
	component: RouteComponent,
});

function RouteComponent() {
	return (
		<RouteLayout>
			<Content>
				<div>
					<div className="mb-4 pt-2 max-w-5xl mx-auto">
						<div className="flex justify-between items-center px-6 @6xl:px-0 py-4">
							<H1>Project Settings</H1>
							<HelpDropdown>
								<Button
									variant="outline"
									startIcon={<Icon icon={faQuestionCircle} />}
								>
									Need help?
								</Button>
							</HelpDropdown>
						</div>
					</div>

					<hr className="mb-6" />

					<div className="px-4">
						<DangerZone />
					</div>
				</div>
			</Content>
		</RouteLayout>
	);
}

function DangerZone() {
	const dataProvider = useCloudProjectDataProvider();
	const navigate = useNavigate();

	const { data: project } = useQuery(
		dataProvider.currentProjectQueryOptions(),
	);

	return (
		<div className="px-6 max-w-5xl mx-auto my-8">
			<section className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
				<header className="flex items-center gap-2 px-5 py-4">
					<Icon
						icon={faTriangleExclamation}
						className="size-3.5 text-destructive"
					/>
					<h3 className="text-sm font-semibold text-foreground">
						Danger zone
					</h3>
				</header>
				<div className="border-t border-foreground/10">
					<div className="flex items-start justify-between gap-4 px-5 py-4">
						<div className="min-w-0">
							<div className="text-sm font-medium text-foreground">
								Archive project
							</div>
							<SmallText className="text-muted-foreground">
								Permanently removes all associated namespaces, Rivet Actors, Runners, and configurations. Cannot be undone.
							</SmallText>
						</div>
						<Button
							variant="destructive-outline"
							size="sm"
							startIcon={<Icon icon={faTrash} />}
							onClick={() =>
								navigate({
									to: ".",
									search: (old) => ({
										...(old as Record<string, unknown>),
										modal: "delete-project",
										displayName: project?.displayName,
									}),
								})
							}
						>
							Archive
						</Button>
					</div>
				</div>
			</section>
		</div>
	);
}
