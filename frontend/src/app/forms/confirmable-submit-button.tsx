import { faTriangleExclamation, Icon } from "@rivet-gg/icons";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { useFormContext, useFormState, useWatch } from "react-hook-form";
import { Button, WithTooltip } from "@/components";

export interface ConfirmableSubmitButtonProps {
	label: string;
	confirmLabel?: string;
	blocked?: boolean;
	blockedReason?: string | null;
	getConfirmation: () =>
		| ReactNode
		| null
		| Promise<ReactNode | null>;
}

export function ConfirmableSubmitButton({
	label,
	confirmLabel,
	blocked = false,
	blockedReason = null,
	getConfirmation,
}: ConfirmableSubmitButtonProps) {
	const form = useFormContext();
	const { isSubmitting, isValidating } = useFormState();
	const [pending, setPending] = useState<ReactNode | null>(null);
	const hiddenSubmitRef = useRef<HTMLButtonElement>(null);

	const allValues = useWatch();
	useEffect(() => {
		setPending(null);
	}, [allValues]);

	const onSubmitClick = async (e: React.MouseEvent<HTMLButtonElement>) => {
		e.preventDefault();
		const valid = await form.trigger();
		if (!valid) return;
		const confirmation = await getConfirmation();
		if (confirmation === null) {
			hiddenSubmitRef.current?.click();
		} else {
			setPending(confirmation);
		}
	};

	const onConfirmClick = (e: React.MouseEvent<HTMLButtonElement>) => {
		e.preventDefault();
		setPending(null);
		hiddenSubmitRef.current?.click();
	};

	const button = (
		<Button
			type="button"
			onClick={onSubmitClick}
			disabled={blocked}
			isLoading={isSubmitting || isValidating}
		>
			{label}
		</Button>
	);

	return (
		<>
			<button
				type="submit"
				ref={hiddenSubmitRef}
				className="hidden"
				tabIndex={-1}
				aria-hidden
			/>
			{pending ? (
				<div className="flex flex-col gap-2 items-stretch">
					<p className="text-sm text-muted-foreground max-w-md self-end text-left">
						<Icon
							icon={faTriangleExclamation}
							className="text-destructive mr-1.5"
						/>
						{pending}
					</p>
					<div className="flex gap-2 self-end">
						<Button
							type="button"
							variant="secondary"
							onClick={() => setPending(null)}
							disabled={isSubmitting}
						>
							Cancel
						</Button>
						<Button
							type="button"
							variant="destructive"
							onClick={onConfirmClick}
							isLoading={isSubmitting || isValidating}
						>
							{confirmLabel ?? `Confirm ${label}`}
						</Button>
					</div>
				</div>
			) : blocked && blockedReason ? (
				<WithTooltip
					trigger={<span>{button}</span>}
					content={blockedReason}
				/>
			) : (
				button
			)}
		</>
	);
}
