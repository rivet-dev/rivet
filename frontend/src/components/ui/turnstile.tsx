import { Turnstile } from "@marsidev/react-turnstile";

interface TurnstileWidgetProps {
	siteKey: string;
	onSuccess: (token: string) => void;
	onExpire: () => void;
	onError: () => void;
}

export function TurnstileWidget({
	siteKey,
	onSuccess,
	onExpire,
	onError,
}: TurnstileWidgetProps) {
	return (
		<Turnstile
			siteKey={siteKey}
			onSuccess={onSuccess}
			onExpire={onExpire}
			onError={onError}
			options={{ appearance: "interaction-only" }}
		/>
	);
}
