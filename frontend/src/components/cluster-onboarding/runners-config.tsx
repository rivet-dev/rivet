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
import { type Cloud, NODE_SIZES, type SizeId, type Tier } from "./catalog";
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
	const perNode = cloud.id === "byoc" || tier.id === "free";
	return (
		<div className="flex flex-col gap-4">
			<RunnerCount
				value={value.count}
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
							<TableHead>Size</TableHead>
							<TableHead className="text-right">vCPU</TableHead>
							<TableHead className="text-right">RAM</TableHead>
							<TableHead className="text-right">
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
											className="cursor-pointer font-mono"
										>
											{size.label}
										</Label>
									</TableCell>
									<TableCell className="py-2.5 text-right text-muted-foreground">
										{size.vcpu.label}
									</TableCell>
									<TableCell className="py-2.5 text-right text-muted-foreground">
										{formatMemory(size.memoryMb)}
									</TableCell>
									<TableCell className="py-2.5 text-right font-medium">
										{perNode
											? tier.id === "free"
												? "Included"
												: "Your cloud"
											: formatUsd(
													nodeMonthlyUsd(size, cloud),
												)}
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
	max,
	onChange,
}: {
	value: number;
	max: number;
	onChange: (count: number) => void;
}) {
	const clamp = (n: number) => Math.min(Math.max(Math.round(n), 1), max);
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
					aria-label="Fewer runners"
					disabled={value <= 1}
					onClick={() => onChange(clamp(value - 1))}
				>
					<Icon icon={faMinus} />
				</Button>
				<Input
					id="runner-count"
					type="number"
					inputMode="numeric"
					min={1}
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
					aria-label="More runners"
					disabled={value >= max}
					onClick={() => onChange(clamp(value + 1))}
				>
					<Icon icon={faPlus} />
				</Button>
			</div>
			<span className="text-xs text-muted-foreground">up to {max}</span>
		</div>
	);
}
