import { GuardEnterprise } from "@/lib/guards";
import { faCircleDollar, faFolder, faGear } from "@rivet-gg/icons";
import { HeaderLink } from "../header-link";

interface HeaderProjectLinksProps {
	projectNameId: string;
}

export function HeaderProjectLinks({ projectNameId }: HeaderProjectLinksProps) {
	return (
		<>
			<HeaderLink
				icon={faFolder}
				to="/projects/$projectNameId"
				activeOptions={{ exact: true }}
				params={{ projectNameId }}
			>
				Environments
			</HeaderLink>
			<GuardEnterprise>
				<HeaderLink
					icon={faCircleDollar}
					to="/projects/$projectNameId/billing"
					params={{ projectNameId }}
				>
					Billing
				</HeaderLink>
			</GuardEnterprise>
			<HeaderLink
				icon={faGear}
				to="/projects/$projectNameId/settings"
				params={{ projectNameId }}
			>
				Settings
			</HeaderLink>
		</>
	);
}
