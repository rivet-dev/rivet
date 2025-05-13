import {
	Button,
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuTrigger,
	WithTooltip,
} from "@rivet-gg/components";
import { Icon, faCog } from "@rivet-gg/icons";
import { useActorDetailsSettings } from "./actor-details-settings";

export function ActorDetailsSettingsButton() {
	const [settings, setSettings] = useActorDetailsSettings();

	return (
		<DropdownMenu>
			<WithTooltip
				trigger={
					<DropdownMenuTrigger asChild>
						<Button
							className="ml-2 place-self-center"
							variant="outline"
							aria-label="Settings"
							size="icon-sm"
						>
							<Icon icon={faCog} />
						</Button>
					</DropdownMenuTrigger>
				}
				content="Settings"
			/>
			<DropdownMenuContent>
				<DropdownMenuCheckboxItem
					checked={settings.showTimestamps}
					onCheckedChange={(value) => {
						setSettings((old) => ({
							...old,
							showTimestamps: value,
						}));
					}}
				>
					Show timestamps
				</DropdownMenuCheckboxItem>
				<DropdownMenuCheckboxItem
					checked={settings.autoFollowLogs}
					onCheckedChange={(value) => {
						setSettings((old) => ({
							...old,
							autoFollowLogs: value,
						}));
					}}
				>
					Auto follow logs when scrolled to bottom
				</DropdownMenuCheckboxItem>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
