import { faCheck, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { motion } from "framer-motion";
import { useEffect, useRef } from "react";
import type { UseFormReturn } from "react-hook-form";
import z from "zod";
import { cn, createSchemaForm } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { NeedHelp } from "./connect-vercel-form";

export const formSchema = z.object({});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const ConnectionCheck = function ConnectionCheck() {
	const { data } = useInfiniteQuery({
		...useEngineCompatDataProvider().runnersQueryOptions(),
		refetchInterval: 1000,
		maxPages: 9999,
		select: (data) =>
			data.pages.reduce((acc, page) => acc + page.runners.length, 0),
	});

	const lastCount = useRef(data);

	useEffect(() => {
		lastCount.current = data;
	}, [data]);

	const success =
		data !== undefined && data > 0 && data !== lastCount.current;

	return (
		<motion.div
			layoutId="msg"
			className={cn(
				"text-center text-muted-foreground text-sm overflow-hidden flex items-center justify-center",
				success && "text-primary-foreground",
			)}
			initial={{ height: 0, opacity: 0.5 }}
			animate={{ height: "6rem", opacity: 1 }}
		>
			{success ? (
				<>
					<Icon icon={faCheck} className="mr-1.5 text-primary" />{" "}
					Runner successfully connected
				</>
			) : (
				<div className="flex flex-col items-center gap-2">
					<div className="flex items-center">
						<Icon
							icon={faSpinnerThird}
							className="mr-1.5 animate-spin"
						/>{" "}
						Waiting for Runner to connect...
					</div>
					<NeedHelp />
				</div>
			)}
		</motion.div>
	);
};
