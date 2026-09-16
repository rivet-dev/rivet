import Cal, { getCalApi } from "@calcom/embed-react";
import { useEffect } from "react";
import { Frame } from "@/components";
import { BYOC_CAL_LINK, BYOC_CAL_NAMESPACE } from "@/content/byoc";
import { useTheme } from "@/lib/theme";

export default function ByocContactFrameContent() {
	const { theme } = useTheme();

	useEffect(() => {
		void (async () => {
			const cal = await getCalApi({ namespace: BYOC_CAL_NAMESPACE });
			cal("ui", {
				theme,
				cssVarsPerTheme: {
					light: { "cal-brand": "#000000" },
					dark: { "cal-brand": "#fafafa" },
				},
				// The dialog card supplies the surface so the embed matches
				// the dashboard instead of painting its own near-black box.
				styles: { body: { background: "transparent" } },
				hideEventTypeDetails: false,
				layout: "month_view",
			});
		})();
	}, [theme]);

	return (
		<>
			<Frame.Header>
				<Frame.Title>Book a call</Frame.Title>
				<Frame.Description>
					Talk to the team about your cluster.
				</Frame.Description>
			</Frame.Header>
			<Frame.Content className="p-0">
				<Cal
					// The embed reads `config.theme` once at init, so remount
					// when the dashboard theme flips while the dialog is open.
					key={theme}
					namespace={BYOC_CAL_NAMESPACE}
					calLink={BYOC_CAL_LINK}
					style={{ width: "100%" }}
					config={{
						theme,
						layout: "month_view",
						useSlotsViewOnSmallScreen: "true",
					}}
				/>
			</Frame.Content>
		</>
	);
}
