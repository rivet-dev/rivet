import {
	faGlobe,
	faPlus,
	faShieldHalved,
	faTrash,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { useForm } from "react-hook-form";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { isValidCidr } from "./model";

const ALLOW_ALL = "0.0.0.0/0";

interface IpAllowlistProps {
	value: string[];
	onChange: (value: string[]) => void;
	/** Compact variant for the dashboard card. */
	compact?: boolean;
}

/**
 * Aiven-style "Allowed inbound IP addresses" editor. Shared by the create
 * page (inside the cluster form) and the dashboard (standalone card).
 */
export function IpAllowlist({ value, onChange, compact }: IpAllowlistProps) {
	const form = useForm<{ cidr: string }>({ defaultValues: { cidr: "" } });
	const allowsEverything = value.includes(ALLOW_ALL);
	const isEmpty = value.length === 0;

	const add = form.handleSubmit(({ cidr }) => {
		const normalized = cidr.trim().includes("/")
			? cidr.trim()
			: `${cidr.trim()}/32`;
		if (!isValidCidr(normalized)) {
			form.setError("cidr", {
				message:
					"Enter an IPv4 address or CIDR block, e.g. 10.20.0.0/16",
			});
			return;
		}
		if (value.includes(normalized)) {
			form.setError("cidr", { message: "Already in the allowlist" });
			return;
		}
		onChange([...value, normalized]);
		// Not `reset()`: that calls the native reset of the closest <form>,
		// which on the create page is the whole cluster form.
		form.resetField("cidr");
	});

	return (
		<div className="flex flex-col gap-3">
			<ul className="flex flex-col divide-y divide-border rounded-lg border border-border">
				{value.map((cidr) => (
					<li
						key={cidr}
						className="flex items-center gap-3 px-3 py-2 text-sm"
					>
						<Icon
							icon={cidr === ALLOW_ALL ? faGlobe : faShieldHalved}
							className={
								cidr === ALLOW_ALL
									? "text-warning"
									: "text-muted-foreground"
							}
						/>
						<span className="font-mono flex-1">{cidr}</span>
						{cidr === ALLOW_ALL ? (
							<Badge variant="warning">Allows all traffic</Badge>
						) : null}
						<Button
							type="button"
							variant="ghost"
							size="icon-sm"
							aria-label={`Remove ${cidr}`}
							onClick={() =>
								onChange(value.filter((v) => v !== cidr))
							}
						>
							<Icon icon={faTrash} />
						</Button>
					</li>
				))}
				{isEmpty ? (
					<li className="flex items-center gap-2 px-3 py-3 text-sm text-destructive">
						<Icon icon={faTriangleExclamation} />
						No addresses allowed. Nothing can connect to this
						cluster until you add a range.
					</li>
				) : null}
			</ul>

			{/* Not a <form>: on the create page this sits inside the cluster
			    form, and nested forms submit the outer one natively. */}
			<div className="flex items-start gap-2">
				<div className="flex-1">
					<Input
						{...form.register("cidr")}
						placeholder="10.20.0.0/16"
						className="font-mono"
						aria-label="IP address or CIDR block"
						aria-invalid={!!form.formState.errors.cidr}
						onKeyDown={(e) => {
							if (e.key === "Enter") {
								e.preventDefault();
								void add();
							}
						}}
					/>
					{form.formState.errors.cidr ? (
						<p className="mt-1 text-xs text-destructive">
							{form.formState.errors.cidr.message}
						</p>
					) : null}
				</div>
				<Button
					type="button"
					variant="outline"
					startIcon={<Icon icon={faPlus} />}
					onClick={() => void add()}
				>
					Add IP address range
				</Button>
			</div>

			{allowsEverything && !compact ? (
				<p className="text-xs text-muted-foreground">
					<span className="text-warning">0.0.0.0/0</span> allows
					connections from any IP address. Remove it and add your
					application servers and CI ranges before going to
					production.
				</p>
			) : null}
		</div>
	);
}
