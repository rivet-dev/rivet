import * as ActorsFiltersForm from "@/domains/project/forms/actors-filters-form";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
	Skeleton,
} from "@rivet-gg/components";
import { type ReactNode, Suspense } from "react";

interface ActorsFiltersSheetProps {
	title: string;
	children?: ReactNode;
	projectId: string;
	environmentId: string;
	onFiltersSubmitted: (values: ActorsFiltersForm.FormValues) => void;
	tags: Record<string, string>;
	showDestroyed: boolean;
}

export function ActorsFiltersSheet({
	title,
	children,
	projectId,
	environmentId,
	tags,
	showDestroyed,
	onFiltersSubmitted,
}: ActorsFiltersSheetProps) {
	return (
		<Sheet>
			<SheetTrigger asChild>{children}</SheetTrigger>
			<SheetContent side="left">
				<SheetHeader>
					<SheetTitle>{title}</SheetTitle>
					<SheetDescription>
						Filter actors by tags and status.
					</SheetDescription>
					<div className="flex gap-4 flex-col">
						<Suspense
							fallback={
								<>
									<Skeleton className="w-full h-8" />
									<Skeleton className="w-full h-8" />
									<Skeleton className="w-full h-8" />
									<Skeleton className="w-full h-8" />
								</>
							}
						>
							<ActorsFiltersForm.Form
								onSubmit={onFiltersSubmitted}
								defaultValues={{
									tags: {},
									showDestroyed: true,
								}}
								values={{ tags, showDestroyed }}
							>
								<ActorsFiltersForm.Tags
									projectId={projectId}
									environmentId={environmentId}
								/>
								<ActorsFiltersForm.ShowDestroyed />
								<div className="flex gap-2 mt-4 items-center justify-end">
									<ActorsFiltersForm.Submit disablePristine>
										Apply
									</ActorsFiltersForm.Submit>

									<ActorsFiltersForm.Reset
										variant="outline"
										type="button"
										onClick={() => {
											onFiltersSubmitted({
												tags: {},
												showDestroyed: true,
											});
										}}
									>
										Reset
									</ActorsFiltersForm.Reset>
								</div>
							</ActorsFiltersForm.Form>
						</Suspense>
					</div>
				</SheetHeader>
			</SheetContent>
		</Sheet>
	);
}
