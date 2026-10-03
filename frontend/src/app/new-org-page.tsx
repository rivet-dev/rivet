import { faXmark, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useMemo, useState } from "react";
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
import { orgConicGradient, paletteForLetter } from "@/lib/org-palette";
import { queryClient } from "@/queries/global";

const formSchema = z.object({
	name: z.string().nonempty("Name cannot be empty"),
});
type FormValues = z.infer<typeof formSchema>;

const { Form, Submit } = createSchemaForm(formSchema);

export function NewOrgPage() {
	const navigate = useNavigate();
	const [nameDraft, setNameDraft] = useState("");
	const palette = useMemo(() => paletteForLetter(nameDraft), [nameDraft]);

	const { mutateAsync, isPending } = useMutation({
		mutationFn: async (values: { name: string }) => {
			const result = await authClient.organization.create({
				name: values.name,
				slug: crypto.randomUUID(),
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
								await mutateAsync(values);
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
								Organizations are shared workspaces where
								teammates collaborate across projects.
							</CardDescription>
						</CardHeader>
						<CardContent className="flex flex-col gap-6">
							<div className="flex justify-center">
								<GradientAvatar
									palette={palette}
									letter={
										nameDraft.trim()[0]?.toUpperCase() ?? ""
									}
								/>
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

function GradientAvatar({
	palette,
	letter,
}: {
	palette: ReturnType<typeof paletteForLetter>;
	letter: string;
}) {
	return (
		<div
			className="flex h-16 w-16 items-center justify-center rounded-full"
			style={{
				backgroundImage: orgConicGradient(palette),
				transition: "background-image 300ms ease",
			}}
		>
			{letter ? (
				<span className="text-2xl font-semibold text-white">
					{letter}
				</span>
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
