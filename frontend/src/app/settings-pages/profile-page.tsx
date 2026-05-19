import { faCamera, faGoogle, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import { type ReactNode, useId, useState } from "react";
import { Avatar, AvatarFallback, AvatarImage, cn, toast } from "@/components";
import { authClient } from "@/lib/auth";
import { resizeImageToDataUrl } from "@/lib/resize-image";

/**
 * Account / Profile tab.
 *
 * Read-only for now: cairo doesn't have an "update name" or "add email"
 * flow wired up, and Security (password / 2FA) isn't wired either, so those
 * surfaces are intentionally omitted to avoid showing controls that go
 * nowhere.
 */
export function ProfilePage() {
	return (
		<div className="space-y-6">
			<ProfileSection />
		</div>
	);
}

function Card({
	title,
	description,
	children,
}: {
	title: string;
	description?: string;
	children: ReactNode;
}) {
	return (
		<div className="rounded-lg border border-foreground/10 bg-card overflow-hidden">
			<div className="px-6 pt-5 pb-4">
				<h3 className="text-sm font-semibold text-foreground">
					{title}
				</h3>
				{description ? (
					<p className="mt-0.5 text-xs text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			<div className="border-t border-foreground/10">{children}</div>
		</div>
	);
}

function Field({
	label,
	children,
	last,
}: {
	label: string;
	children: ReactNode;
	last?: boolean;
}) {
	return (
		<div
			className={cn(
				"grid grid-cols-[160px_1fr] items-center gap-4 px-6 py-3.5 text-sm",
				!last && "border-b border-foreground/10",
			)}
		>
			<div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
				{label}
			</div>
			<div className="min-w-0">{children}</div>
		</div>
	);
}

function ProfileImageUpload({
	image,
	initials,
}: {
	image: string | null;
	initials: string;
}) {
	const inputId = useId();
	const [isUploading, setIsUploading] = useState(false);

	const { mutateAsync } = useMutation({
		mutationFn: async (nextImage: string) => {
			const result = await authClient.updateUser({ image: nextImage });
			if (result.error) throw result.error;
		},
	});

	const onPick = async (file: File) => {
		setIsUploading(true);
		try {
			const dataUrl = await resizeImageToDataUrl(file);
			await mutateAsync(dataUrl);
			toast.success("Profile image updated");
		} catch {
			toast.error("Couldn't update the image. Try a different file.");
		} finally {
			setIsUploading(false);
		}
	};

	return (
		<div className="relative shrink-0">
			<label
				htmlFor={inputId}
				className="block cursor-pointer rounded-full focus-within:outline-none focus-within:ring-2 focus-within:ring-ring"
				aria-label={image ? "Replace profile image" : "Choose profile image"}
			>
				<Avatar className="size-8">
					<AvatarImage src={image ?? undefined} />
					<AvatarFallback className="text-xs">
						{initials}
					</AvatarFallback>
				</Avatar>
				<span className="absolute -bottom-0.5 -right-0.5 flex h-4 w-4 items-center justify-center rounded-full border border-background bg-foreground text-background shadow-sm">
					<Icon icon={faCamera} className="size-2" />
				</span>
			</label>
			<input
				id={inputId}
				type="file"
				accept="image/*"
				className="sr-only"
				disabled={isUploading}
				onChange={(e) => {
					const file = e.target.files?.[0];
					if (file) void onPick(file);
					e.target.value = "";
				}}
			/>
		</div>
	);
}

function ProfileSection() {
	const { data: session } = authClient.useSession();
	const user = session?.user;
	const name = user?.name ?? user?.email?.split("@")[0] ?? "Account";
	const email = user?.email ?? "";
	const initials = name
		.split(" ")
		.map((part) => part[0])
		.slice(0, 2)
		.join("")
		.toUpperCase();

	return (
		<Card
			title="Profile details"
			description="Your personal information and connected identities."
		>
			<Field label="Profile">
				<div className="flex items-center gap-3 min-w-0">
					<ProfileImageUpload
						image={user?.image ?? null}
						initials={initials || "?"}
					/>
					<span className="text-sm text-foreground truncate">
						{name}
					</span>
				</div>
			</Field>
			<Field label="Email">
				<span className="text-sm text-foreground truncate">
					{email}
				</span>
			</Field>
			<Field label="Connected accounts" last>
				<div className="flex items-center gap-2 min-w-0">
					<Icon
						icon={faGoogle}
						className="size-3.5 text-muted-foreground shrink-0"
					/>
					<span className="text-sm text-foreground">Google</span>
					<span className="text-muted-foreground/50 text-sm">·</span>
					<span className="text-sm text-muted-foreground truncate">
						{email}
					</span>
				</div>
			</Field>
		</Card>
	);
}
