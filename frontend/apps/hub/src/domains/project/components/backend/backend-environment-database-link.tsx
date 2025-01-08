import { Slot } from "@radix-ui/react-slot";
import { Button, type ButtonProps } from "@rivet-gg/components";
import { Icon, faDatabase, faExternalLink } from "@rivet-gg/icons";
import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, forwardRef } from "react";
import { useEnvironmentDatabasePreview } from "../../queries";

interface BackendEnvironmentDatabaseLinkProps
	extends Omit<ButtonProps, "onClick"> {
	projectId: string;
	environmentId: string;
	asChild?: boolean;
	children?: ReactNode;
}

export const BackendEnvironmentDatabaseLink = forwardRef<
	HTMLButtonElement,
	BackendEnvironmentDatabaseLinkProps
>(({ projectId, environmentId, asChild, children, ...props }, ref) => {
	const { isLoading, data } = useEnvironmentDatabasePreview({
		projectId,
		environmentId,
	});

	const C = asChild ? Slot : Button;
	const navigate = useNavigate();

	return (
		<C
			ref={ref}
			onClick={async (e) => {
				e?.preventDefault();
				if (isLoading) {
					return;
				}
				if (!data) {
					return navigate({ to: ".", search: { modal: "database" } });
				}
				if (typeof data === "string") {
					window.open(data, "_blank", "noreferrer,noopener");
				}
			}}
			isLoading={isLoading}
			startIcon={<Icon icon={faDatabase} className={"size-4"} />}
			endIcon={
				data ? (
					<Icon icon={faExternalLink} className={"size-4"} />
				) : undefined
			}
			{...props}
		>
			{children}
		</C>
	);
});
