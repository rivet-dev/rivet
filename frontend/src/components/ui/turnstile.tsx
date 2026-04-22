import { Turnstile, TurnstileInstance } from "@marsidev/react-turnstile";

interface TurnstileWidgetProps {
	ref?: React.Ref<TurnstileInstance>;
	siteKey: string;
	onSuccess: (token: string) => void;
	onExpire: () => void;
	onError: () => void;
	onTimeout?: () => void;
}

export function TurnstileWidget({
	ref,
	siteKey,
	onSuccess,
	onExpire,
	onError,
	onTimeout,
}: TurnstileWidgetProps) {
	return (
		<Turnstile
			ref={ref}
			siteKey={siteKey}
			onSuccess={onSuccess}
			onExpire={onExpire}
			onError={onError}
			onTimeout={onTimeout}
			options={{ appearance: "interaction-only" }}
		/>
	);
}
