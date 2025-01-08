import { GuardEnterprise } from "@/lib/guards";
import { faCircleDollar, faFolder, faGear } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";
import { HeaderLink } from "../header-link";

interface HeaderProjectLinksProps {
	projectNameId: string;
}

export function HeaderProjectLinks({ projectNameId }: HeaderProjectLinksProps) {
	return (
		<>
			<HeaderLink icon={faFolder}>
				<Link
					to="/projects/$projectNameId"
					activeOptions={{ exact: true }}
					params={{ projectNameId }}
				>
					Environments
				</Link>
			</HeaderLink>
			<GuardEnterprise>
				<HeaderLink icon={faCircleDollar}>
					<Link
						to="/projects/$projectNameId/billing"
						params={{ projectNameId }}
					>
						Billing
					</Link>
				</HeaderLink>
			</GuardEnterprise>
			<HeaderLink icon={faGear}>
				<Link
					to="/projects/$projectNameId/settings"
					params={{ projectNameId }}
				>
					Settings
				</Link>
			</HeaderLink>
		</>
	);
}
