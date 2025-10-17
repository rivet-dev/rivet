import { faQuestionCircle, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import { HelpDropdown } from "@/app/help-dropdown";
import {
	Button,
	type DialogContentProps,
	DiscreteInput,
	Frame,
	Label,
	Skeleton,
} from "@/components";

interface TokensFrameContentProps extends DialogContentProps {}

export default function TokensFrameContent({
	onClose,
}: TokensFrameContentProps) {
	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>Namespace Tokens</div>
					<HelpDropdown>
						<Button variant="ghost" size="icon">
							<Icon icon={faQuestionCircle} />
						</Button>
					</HelpDropdown>
				</Frame.Title>
				<Frame.Description>
					These tokens are used to authenticate requests to the Rivet
					API.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content>
				<div className="items-center grid grid-cols-1 gap-6">
					<SecretToken />
					<PublishableToken />
				</div>
			</Frame.Content>
			<Frame.Footer>
				<Button variant="secondary" onClick={onClose}>
					Close
				</Button>
			</Frame.Footer>
		</>
	);
}

function SecretToken() {
	const dataProvider = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
		select: (c) => c.dataProvider,
	});
	const { data, isLoading } = useQuery(
		dataProvider.engineAdminTokenQueryOptions(),
	);
	return (
		<div className="space-y-2">
			<Label>Secret Token</Label>
			{isLoading ? (
				<Skeleton className="w-full h-10" />
			) : (
				<DiscreteInput value={data || ""} />
			)}
			<p className="text-sm text-muted-foreground">
				Only use in secure server environments. Grants full access to
				your namespace. Used to connect your Runners to your namespace.
			</p>
		</div>
	);
}

function PublishableToken() {
	const dataProvider = useRouteContext({
		from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
		select: (c) => c.dataProvider,
	});
	const { data, isLoading } = useQuery(
		dataProvider.publishableTokenQueryOptions(),
	);
	return (
		<div className="space-y-2">
			<Label>Publishable Token</Label>
			{isLoading ? (
				<Skeleton className="w-full h-10" />
			) : (
				<DiscreteInput value={data || ""} />
			)}
			<p className="text-sm text-muted-foreground">
				Safe to use in public contexts like client-side code. Allows
				your frontend to interact with Rivet services.
			</p>
		</div>
	);
}
