import { getCalApi } from "@calcom/embed-react";
import type { ReactNode } from "react";
import { BYOC_CAL_LINK, BYOC_CAL_NAMESPACE } from "@/content/byoc";
import { type Theme, useTheme } from "@/lib/theme";
import { readCalVars } from "./cal-theme";

const CAL_LAYOUT = "month_view" as const;

function buildUiConfig(theme: Theme) {
	const cssVars = readCalVars();
	return {
		theme,
		cssVarsPerTheme: { light: cssVars, dark: cssVars },
		hideEventTypeDetails: false,
		layout: CAL_LAYOUT,
	};
}

let latestUiConfig: ReturnType<typeof buildUiConfig> | null = null;
let listening = false;

export function ByocContactTrigger({
	children,
}: {
	children: (open: () => void) => ReactNode;
}) {
	const { theme } = useTheme();

	const open = () => {
		void (async () => {
			for (const stale of document.querySelectorAll("cal-modal-box")) {
				stale.remove();
			}
			const cal = await getCalApi({ namespace: BYOC_CAL_NAMESPACE });
			latestUiConfig = buildUiConfig(theme);
			if (!listening) {
				listening = true;
				cal("on", {
					action: "linkReady",
					callback: () => {
						if (latestUiConfig) {
							cal("ui", latestUiConfig);
						}
					},
				});
			}
			cal("ui", latestUiConfig);
			cal("modal", {
				calLink: BYOC_CAL_LINK,
				config: { theme, layout: CAL_LAYOUT },
			});
		})();
	};

	return <>{children(open)}</>;
}
