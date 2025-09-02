import { faCog, Icon } from "@rivet-gg/icons";
import {
	Button,
	cn,
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuTrigger,
	WithTooltip,
} from "@/components";
import { useActorDetailsSettings } from "./actor-details-settings";

interface ActorDetailsSettingsButtonProps {
	className?: string;
}

export function ActorDetailsSettingsButton({
	className,
}: ActorDetailsSettingsButtonProps) {
	const [settings, setSettings] = useActorDetailsSettings();

	return (
		<DropdownMenu>
			<WithTooltip
				trigger={
					<DropdownMenuTrigger asChild>
						<Button
							className={cn("place-self-center", className)}
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
