import { faCamera, faXmark, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useId, useMemo, useState } from "react";
import { useFormContext } from "react-hook-form";
import z from "zod";
import {
	Button,
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	cn,
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";
import { authClient } from "@/lib/auth";
import { paletteForLetter } from "@/lib/org-palette";
import { resizeImageToDataUrl } from "@/lib/resize-image";
import { queryClient } from "@/queries/global";

const formSchema = z.object({
	name: z.string().nonempty("Name cannot be empty"),
});
type FormValues = z.infer<typeof formSchema>;

const { Form, Submit } = createSchemaForm(formSchema);

export function NewOrgPage() {
	const navigate = useNavigate();
	const [nameDraft, setNameDraft] = useState("");
	const [logo, setLogo] = useState<string | null>(null);
	const [logoError, setLogoError] = useState<string | null>(null);
	const palette = useMemo(() => paletteForLetter(nameDraft), [nameDraft]);

	const { mutateAsync, isPending } = useMutation({
		mutationFn: async (values: { name: string; logo: string | null }) => {
			const result = await authClient.organization.create({
				name: values.name,
				slug: crypto.randomUUID(),
				logo: values.logo ?? undefined,
			});
			if (result.error) throw result.error;
			return result;
		},
		onSuccess: async (data) => {
			await queryClient.invalidateQueries({
				queryKey: ["organizations"],
			});
			await navigate({
				to: "/orgs/$organization",
				params: { organization: data.data.slug },
			});
		},
	});

	return (
		<div
			className="relative flex min-h-screen w-full flex-col bg-background text-foreground"
			style={
				{
					"--org-c1": palette.c1,
					"--org-c2": palette.c2,
					"--org-c3": palette.c3,
					"--org-c4": palette.c4,
					"--org-accent": palette.accent,
				} as React.CSSProperties
			}
		>
			<div className="absolute right-4 top-4 z-10">
				<Button
					type="button"
					variant="ghost"
					size="icon"
					onClick={() => navigate({ to: "/" })}
					aria-label="Close"
				>
					<Icon icon={faXmark} className="size-4" />
				</Button>
			</div>

			<div className="flex flex-1 items-center justify-center px-6">
				<Card className="w-full max-w-md">
					<Form
						defaultValues={{ name: "" }}
						mode="onSubmit"
						revalidateMode="onSubmit"
						onSubmit={async (values, form) => {
							try {
								await mutateAsync({ ...values, logo });
							} catch {
								form.setError("root", {
									message:
										"Couldn't create the organization. Try a different name.",
								});
							}
						}}
					>
						<CardHeader>
							<CardTitle>Create a new organization</CardTitle>
							<CardDescription>
								Organizations are shared workspaces where teammates
								collaborate across projects.
							</CardDescription>
						</CardHeader>
						<CardContent className="flex flex-col gap-6">
							<div className="flex flex-col items-center gap-2">
								<AvatarUpload
									palette={palette}
									letter={
										nameDraft.trim()[0]?.toUpperCase() ?? ""
									}
									logo={logo}
									onPick={async (file) => {
										setLogoError(null);
										try {
											const dataUrl =
												await resizeImageToDataUrl(file);
											setLogo(dataUrl);
										} catch {
											setLogoError(
												"Couldn't read that image. Try a different file.",
											);
										}
									}}
									onClear={() => {
										setLogo(null);
										setLogoError(null);
									}}
								/>
								{logoError ? (
									<p className="text-xs text-destructive">
										{logoError}
									</p>
								) : null}
							</div>
							<NameField onValueChange={setNameDraft} />
							<RootError />
						</CardContent>
						<CardFooter className="justify-end">
							<Submit type="submit" isLoading={isPending}>
								Continue
							</Submit>
						</CardFooter>
					</Form>
				</Card>
			</div>
		</div>
	);
}

function AvatarUpload({
	palette,
	letter,
	logo,
	onPick,
	onClear,
}: {
	palette: ReturnType<typeof paletteForLetter>;
	letter: string;
	logo: string | null;
	onPick: (file: File) => void;
	onClear: () => void;
}) {
	const inputId = useId();

	return (
		<div className="flex flex-col items-center gap-2">
			<div className="relative">
				<label
					htmlFor={inputId}
					className="block cursor-pointer rounded-full focus-within:outline-none focus-within:ring-2 focus-within:ring-ring"
					aria-label={logo ? "Replace avatar" : "Choose avatar"}
				>
					{logo ? (
						<div className="relative h-16 w-16 overflow-hidden rounded-full">
							<img
								src={logo}
								alt=""
								className="h-full w-full object-cover"
							/>
						</div>
					) : (
						<GradientAvatar palette={palette} letter={letter} />
					)}
					<span className="absolute -bottom-0.5 -right-0.5 flex h-6 w-6 items-center justify-center rounded-full border border-background bg-foreground text-background shadow-sm">
						<Icon icon={faCamera} className="size-3" />
					</span>
				</label>
				<input
					id={inputId}
					type="file"
					accept="image/*"
					className="sr-only"
					onChange={(e) => {
						const file = e.target.files?.[0];
						if (file) onPick(file);
						// Reset so picking the same file again still fires.
						e.target.value = "";
					}}
				/>
			</div>
			{logo ? (
				<button
					type="button"
					onClick={onClear}
					className="text-xs text-muted-foreground underline-offset-2 hover:underline"
				>
					Remove
				</button>
			) : (
				<span className="text-xs text-muted-foreground">
					Choose avatar
				</span>
			)}
		</div>
	);
}

function GradientAvatar({
	palette,
	letter,
}: {
	palette: ReturnType<typeof paletteForLetter>;
	letter: string;
}) {
	return (
		<div className="relative h-16 w-16 animate-orb-breathe">
			<div className="absolute inset-0 overflow-hidden rounded-full">
				<div
					className="absolute -inset-2 animate-conic-rotate"
					style={{
						background: `conic-gradient(from var(--gradient-angle, 0deg) at 50% 50%, ${palette.c1}, ${palette.c2}, ${palette.c3}, ${palette.c4}, ${palette.c2}, ${palette.c1})`,
						filter: "blur(4px) saturate(0.95) contrast(1.05)",
						transition: "background 600ms ease",
					}}
				/>
				<div
					className="absolute -inset-2 animate-conic-rotate-reverse"
					style={{
						background: `conic-gradient(from var(--gradient-angle, 0deg) at 50% 50%, transparent, ${palette.accent}, transparent, ${palette.c4}, transparent)`,
						filter: "blur(10px)",
						mixBlendMode: "overlay",
						opacity: 0.55,
						transition: "background 600ms ease",
					}}
				/>
				<div
					className="absolute inset-0 pointer-events-none"
					style={{
						background:
							"radial-gradient(circle at 30% 22%, hsl(0 0% 100% / 0.4) 0%, hsl(0 0% 100% / 0) 38%), radial-gradient(circle at 72% 82%, hsl(0 0% 0% / 0.45) 0%, hsl(0 0% 0% / 0) 65%)",
					}}
				/>
			</div>
			{letter ? (
				<div className="absolute inset-0 flex items-center justify-center pointer-events-none">
					<span className="text-2xl font-semibold text-white">
						{letter}
					</span>
				</div>
			) : null}
		</div>
	);
}

function NameField({
	onValueChange,
}: {
	onValueChange: (value: string) => void;
}) {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem className="space-y-1.5">
					<FormLabel className="text-xs font-normal text-muted-foreground">
						Organization name
					</FormLabel>
					<FormControl>
						<Input
							placeholder="Acme"
							autoFocus
							autoComplete="off"
							{...field}
							onChange={(e) => {
								field.onChange(e);
								onValueChange(e.target.value);
							}}
							className={cn("h-10 text-sm")}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
}

function RootError() {
	const { formState } = useFormContext<FormValues>();
	if (!formState.errors.root) return null;
	return (
		<p className="text-sm text-destructive">
			{formState.errors.root.message}
		</p>
	);
}
