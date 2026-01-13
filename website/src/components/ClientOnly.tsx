"use client";
import { useState, useEffect, type ReactNode } from "react";

interface ClientOnlyProps {
	children: ReactNode;
	fallback?: ReactNode;
}

export function ClientOnly({ children, fallback = null }: ClientOnlyProps) {
	const [isClient, setIsClient] = useState(false);

	useEffect(() => {
		setIsClient(true);
	}, []);

	if (!isClient) {
		return <>{fallback}</>;
	}

	return <>{children}</>;
}
