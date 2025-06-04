import {
	Button,
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	Flex,
	SmallText,
	Text,
	cn,
} from "@rivet-gg/components";
import { Icon, type IconProp } from "@rivet-gg/icons";
import type { ReactNode } from "@tanstack/react-router";

export interface BillingPlanCardProps {
	title: string;
	lead?: string;
	price: string;
	priceLead?: ReactNode;
	features: {
		key?: string;
		name: ReactNode;
		icon?: IconProp;
		bold?: boolean;
	}[];
	type?: "custom" | "active";
	onSubscribe?: () => void;
	onCancel?: () => void;
	cancelLabel?: string;
	showPrice?: boolean;
	className?: string;
}

export function BillingPlanCard({
	title,
	lead,
	price,
	priceLead,
	type,
	features,
	cancelLabel,
	showPrice = true,
	className,
	onSubscribe,
	onCancel,
}: BillingPlanCardProps) {
	return (
		<Card
			className={cn(
				"flex flex-col group bg-transparent transition-colors",
				onSubscribe || onCancel || type === "custom"
					? "hover:border-primary"
					: "",
				className,
			)}
		>
			<CardHeader>
				<Text className="font-semibold text-2xl">{title}</Text>
				{lead ? <SmallText>{lead}</SmallText> : null}
				{showPrice ? (
					<div className="pt-8 min-h-[7rem]">
						<p>
							<span className="text-5xl font-bold mr-1 ">
								{price}
							</span>
							<span className="text-muted-foreground">
								{type !== "custom" ? "/mo" : null}
							</span>
						</p>
						{priceLead ? (
							<SmallText className="text-muted-foreground">
								{priceLead}
							</SmallText>
						) : null}
					</div>
				) : null}
			</CardHeader>
			<CardContent className="flex-1">
				<Flex direction="col" gap="4" asChild>
					<ul>
						{features.map(({ key, name, bold, icon }) => (
							<Flex
								items="center"
								gap="2"
								key={key || name}
								asChild
							>
								<li
									className={cn(
										bold && "font-semibold",
										bold
											? "text-foreground"
											: "text-muted-foreground",
									)}
								>
									{icon ? (
										<Icon icon={icon} className="size-5 " />
									) : null}
									<span>{name}</span>
								</li>
							</Flex>
						))}
					</ul>
				</Flex>
			</CardContent>
			<CardFooter className="justify-center empty:hidden">
				{type === "custom" ? (
					<a
						href="https://rivet.gg/sales"
						target="_blank"
						rel="noreferrer"
					>
						<Button>Contact</Button>
					</a>
				) : null}
				{type === "active" && onCancel ? (
					<Button variant="secondary" onClick={onCancel}>
						{cancelLabel || "Cancel"}
					</Button>
				) : null}
				{!type && onSubscribe ? (
					<Button onClick={onSubscribe}>Upgrade</Button>
				) : null}
			</CardFooter>
		</Card>
	);
}
