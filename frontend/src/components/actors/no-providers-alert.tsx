import { faBook, faExclamationTriangle, faPlus, Icon } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { ProviderDropdown } from "@/app/provider-dropdown";
import { docsLinks } from "@/content/data";
import { features } from "@/lib/features";
import { Button } from "../ui/button";
import { H4 } from "../ui/typography";

export function NoProvidersAlert({
	variant = "default",
}: {
	variant?: "default" | "connect";
}) {
	return (
		<div className="relative rounded-lg border border-primary p-4 pt-5 flex gap-6 w-full justify-between items-center">
			<div>
				<H4>
					<Icon
						icon={faExclamationTriangle}
						className="mr-2 text-primary"
					/>
					No Providers Connected
				</H4>
				<p className="text-sm text-muted-foreground mt-1 mb-2 pl-7">
					You can't run any Actors yet. Use provider of your choice to
					connect and start deploying and running Rivet Actors.
				</p>
			</div>
			<div className="flex flex-col items-center justify-center gap-2">
				{variant === "default" ? (
					<>
						{features.multitenancy ? (
							<Button size="sm" asChild className="w-full">
								<Link
									to="/orgs/$organization/projects/$project/ns/$namespace/settings"
									from="/orgs/$organization/projects/$project/ns/$namespace"
								>
									Go to Settings
								</Link>
							</Button>
						) : null}
						{!features.multitenancy ? (
							<Button asChild size="sm" className="w-full">
								<Link
									to="/ns/$namespace"
									from="/ns/$namespace/settings"
								>
									Go to Settings
								</Link>
							</Button>
						) : null}
					</>
				) : (
					<ProviderDropdown>
						<Button
							startIcon={<Icon icon={faPlus} />}
							size="sm"
							className="w-full"
						>
							Connect Provider
						</Button>
					</ProviderDropdown>
				)}
				<Button
					startIcon={<Icon icon={faBook} />}
					size="sm"
					asChild
					variant="ghost"
				>
					<a
						href={docsLinks.runnersSetup}
						target="_blank"
						rel="noreferrer"
					>
						Documentation
					</a>
				</Button>
			</div>
		</div>
	);
}
