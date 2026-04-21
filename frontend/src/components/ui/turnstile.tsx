import { Turnstile } from "@marsidev/react-turnstile";

interface TurnstileWidgetProps {
	siteKey: string;
	onSuccess: (token: string) => void;
	onExpire: () => void;
	onError: () => void;
	onTimeout?: () => void;
}

export function TurnstileWidget({
	siteKey,
	onSuccess,
	onExpire,
	onError,
	onTimeout,
}: TurnstileWidgetProps) {
	return (
		<Turnstile
			siteKey={siteKey}
			onSuccess={onSuccess}
			onExpire={onExpire}
			onError={onError}
			onTimeout={onTimeout}
			options={{ appearance: "interaction-only" }}
		/>
	);
}
