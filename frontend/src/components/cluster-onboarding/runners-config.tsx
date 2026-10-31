import { faMinus, faPlus, Icon } from "@rivet-gg/icons";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import {
	type Cloud,
	formatVcpu,
	NODE_SIZES,
	type SizeId,
	type Tier,
} from "./catalog";
import {
	formatMemory,
	formatUsd,
	isSizeAvailable,
	nodeMonthlyUsd,
	type RunnerConfig,
} from "./model";

const RADIO_CLASS =
	"border-foreground/40 text-foreground data-[state=checked]:border-foreground";

interface RunnersConfigProps {
	tier: Tier;
	cloud: Cloud;
	value: RunnerConfig;
	onChange: (value: RunnerConfig) => void;
}

/**
 * Runner node size and count. Shared by the create page and Settings so
 * scaling later looks like what you picked at creation.
 */
export function RunnersConfig({
	tier,
	cloud,
	value,
	onChange,
}: RunnersConfigProps) {
	return (
		<div className="flex flex-col gap-4">
			<RunnerCount
				value={value.count}
				min={tier.minRunners}
				max={tier.maxRunners}
				onChange={(count) => onChange({ ...value, count })}
			/>
			<RadioGroup
				value={value.size}
				onValueChange={(size) =>
					onChange({ ...value, size: size as SizeId })
				}
				className="block"
			>
				<Table containerClassName="rounded-lg border border-border">
					<TableHeader>
						<TableRow className="hover:bg-transparent">
							<TableHead className="w-10 pr-0" />
							<TableHead className="w-32">vCPU</TableHead>
							<TableHead>RAM</TableHead>
							<TableHead className="whitespace-nowrap text-right">
								Per node / month
							</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody className="tabular-nums">
						{NODE_SIZES.filter((size) =>
							isSizeAvailable(tier.id, size.id),
						).map((size) => {
							const selected = value.size === size.id;
							return (
								<TableRow
									key={size.id}
									data-state={
										selected ? "selected" : undefined
									}
									className="cursor-pointer"
								>
									<TableCell className="py-2.5 pr-0">
										<RadioGroupItem
											id={`runner-size-${size.id}`}
											value={size.id}
											className={RADIO_CLASS}
										/>
									</TableCell>
									<TableCell className="py-2.5">
										<Label
											htmlFor={`runner-size-${size.id}`}
											className="cursor-pointer"
										>
											{formatVcpu(size.vcpu)}
										</Label>
									</TableCell>
									<TableCell className="py-2.5 text-muted-foreground">
										{formatMemory(size.memoryMb)}
									</TableCell>
									<TableCell className="py-2.5 text-right font-medium">
										{formatUsd(nodeMonthlyUsd(size, cloud))}
									</TableCell>
								</TableRow>
							);
						})}
					</TableBody>
				</Table>
			</RadioGroup>
		</div>
	);
}

function RunnerCount({
	value,
	min,
	max,
	onChange,
}: {
	value: number;
	min: number;
	max?: number;
	onChange: (count: number) => void;
}) {
	const clamp = (n: number) =>
		Math.min(Math.max(Math.round(n), min), max ?? Number.POSITIVE_INFINITY);
	const hint =
		max === undefined
			? min > 1
				? `at least ${min}`
				: null
			: min > 1
				? `${min} to ${max}`
				: `up to ${max}`;
	return (
		<div className="flex items-center gap-3">
			<Label htmlFor="runner-count" className="text-sm">
				Nodes
			</Label>
			<div className="flex items-center">
				<Button
					type="button"
					variant="outline"
					size="icon-sm"
					className="rounded-r-none"
					aria-label="Fewer actor nodes"
					disabled={value <= min}
					onClick={() => onChange(clamp(value - 1))}
				>
					<Icon icon={faMinus} />
				</Button>
				<Input
					id="runner-count"
					type="number"
					inputMode="numeric"
					min={min}
					max={max}
					value={value}
					onChange={(e) => {
						const next = Number(e.target.value);
						if (Number.isFinite(next)) onChange(clamp(next));
					}}
					className="h-7 w-16 rounded-none border-x-0 text-center font-mono tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
				/>
				<Button
					type="button"
					variant="outline"
					size="icon-sm"
					className="rounded-l-none"
					aria-label="More actor nodes"
					disabled={max !== undefined && value >= max}
					onClick={() => onChange(clamp(value + 1))}
				>
					<Icon icon={faPlus} />
				</Button>
			</div>
			{hint ? (
				<span className="text-xs text-muted-foreground">{hint}</span>
			) : null}
		</div>
	);
}
