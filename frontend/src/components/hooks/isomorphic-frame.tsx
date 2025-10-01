import { createContext, useContext } from "react";
import {
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components";

export const IsInModalContext = createContext(false);

export const Header = (props: React.ComponentProps<typeof DialogHeader>) => {
	const isInModal = useContext(IsInModalContext);
	return isInModal ? <DialogHeader {...props} /> : <CardHeader {...props} />;
};

export const Title = (props: React.ComponentProps<typeof DialogTitle>) => {
	const isInModal = useContext(IsInModalContext);
	return isInModal ? <DialogTitle {...props} /> : <CardTitle {...props} />;
};

export const Content = (props: React.HTMLAttributes<HTMLDivElement>) => {
	const isInModal = useContext(IsInModalContext);
	return isInModal ? (
		<DialogContent {...props} />
	) : (
		<CardContent {...props} />
	);
};

export const Footer = (props: React.ComponentProps<typeof DialogFooter>) => {
	const isInModal = useContext(IsInModalContext);
	return isInModal ? <DialogFooter {...props} /> : <CardFooter {...props} />;
};
