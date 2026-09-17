import Cal, { getCalApi } from "@calcom/embed-react";
import { useEffect } from "react";
import { Frame } from "@/components";
import { BYOC_CAL_LINK, BYOC_CAL_NAMESPACE } from "@/content/byoc";

export default function ByocContactFrameContent() {
	useEffect(() => {
		void (async () => {
			const cal = await getCalApi({ namespace: BYOC_CAL_NAMESPACE });
			cal("ui", {
				cssVarsPerTheme: {
					light: { "cal-brand": "#000000" },
					dark: { "cal-brand": "#fafafa" },
				},
				hideEventTypeDetails: false,
				layout: "month_view",
			});
		})();
	}, []);

	return (
		<Frame.Content className="p-0 pt-10">
			<Cal
				namespace={BYOC_CAL_NAMESPACE}
				calLink={BYOC_CAL_LINK}
				style={{ width: "100%" }}
				config={{
					layout: "month_view",
					useSlotsViewOnSmallScreen: "true",
				}}
			/>
		</Frame.Content>
	);
}
