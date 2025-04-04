import * as ActorsFiltersForm from "./form/actors-filters-form";
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
	Skeleton,
} from "@rivet-gg/components";
import { useAtomValue } from "jotai";
import { type ReactNode, Suspense } from "react";
import { actorFiltersAtom } from "./actor-context";

interface ActorsFiltersSheetProps {
	onFiltersSubmitted: (values: ActorsFiltersForm.FormValues) => void;
	children: ReactNode;
}

export function ActorsFiltersSheet({
	onFiltersSubmitted,
	children,
}: ActorsFiltersSheetProps) {
	const { tags, showDestroyed } = useAtomValue(actorFiltersAtom);
	return (
		<Sheet>
			<SheetTrigger asChild>{children}</SheetTrigger>
			<SheetContent side="left">
				<SheetHeader>
					<SheetTitle>Filters</SheetTitle>
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
								<ActorsFiltersForm.Tags />
								<ActorsFiltersForm.ShowDestroyed />
								<div className="flex gap-2 mt-4 items-center justify-end">
									<ActorsFiltersForm.Submit>
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
