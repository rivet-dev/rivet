"use client";

import dynamic from "next/dynamic";

// Giscus causes hydration errors in Next.js 15 due to its internal use of
// pathname resolution conflicting with RSC streaming. Disabling SSR prevents
// the "can't access property 'split', file is undefined" error in encodeURIPath.
const Comments = dynamic(
	() => import("@/components/Comments").then((mod) => mod.Comments),
	{ ssr: false }
);

export function CommentsWrapper() {
	return <Comments />;
}
