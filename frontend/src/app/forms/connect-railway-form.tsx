import * as ConnectManualServerlfullForm from "@/app/forms/connect-manual-serverfull-form";

export const RunnerName = ConnectManualServerlfullForm.RunnerName;
export const Datacenter = () => {
	return (
		<ConnectManualServerlfullForm.Datacenter
			message={
				<>
					You can find the region your Railway runners are running in
					under <i>Settings &gt; Deploy</i>
				</>
			}
		/>
	);
};

export const ConnectionCheck = ConnectManualServerlfullForm.ConnectionCheck;
