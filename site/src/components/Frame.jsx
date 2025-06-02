import { clsx } from "clsx";
import React from "react";

export function Frame({ caption, className, children }) {
	return (
		<div className={clsx(className, "mx-auto py-6")}>
			<div className="not-prose relative bg-white/5 rounded-xl overflow-hidden border border-white/10 overflow-hidden">
				<div className="relative flex justify-center border-b border-white/5">
					{children}
				</div>
				{caption && (
					<div className="relative rounded-xl overflow-auto flex justify-center text-sm text-slate-400 px-4 py-4">
						<p>{caption}</p>
					</div>
				)}
			</div>
		</div>
	);
}
