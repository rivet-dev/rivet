import type { Rivet } from "@rivet-gg/api";
import { useMutation } from "@tanstack/react-query";
import { queryClient, rivetClient } from "../../../queries/global";

export const useStartEmailVerificationMutation = () => {
	return useMutation({
		mutationFn: (data: Rivet.auth.identity.StartEmailVerificationRequest) =>
			rivetClient.auth.identity.email.startEmailVerification(data),
	});
};

export const useCompleteEmailVerificationMutation = (
	opts: {
		onSuccess?: (
			data: Rivet.auth.identity.CompleteEmailVerificationResponse,
		) => void;
	} = {},
) => {
	return useMutation({
		mutationFn: (
			data: Rivet.auth.identity.CompleteEmailVerificationRequest,
		) => rivetClient.auth.identity.email.completeEmailVerification(data),
		...opts,
	});
};

export const deviceLinkTokenQueryOptions = (deviceLinkToken: string) => {
	return {
		queryKey: ["deviceLinkToken", deviceLinkToken],
		queryFn: () => rivetClient.cloud.devices.links.get({ deviceLinkToken }),
	};
};

export const useCompleteDeviceLinkMutation = () => {
	return useMutation({
		mutationFn: (
			data: Rivet.cloud.devices.links.CompleteDeviceLinkRequest,
		) => rivetClient.cloud.devices.links.complete(data),
		onSuccess: (_, values) => {
			queryClient.invalidateQueries(
				deviceLinkTokenQueryOptions(values.deviceLinkToken),
			);
		},
	});
};
