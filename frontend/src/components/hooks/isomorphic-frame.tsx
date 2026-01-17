import { DialogDescription } from "@radix-ui/react-dialog";
import { createContext, useContext } from "react";
import {
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	cn,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components";

export const IsInModalContext = createContext(false);
export const FrameConfigContext = createContext<{
	showHeader?: boolean;
	showFooter?: boolean;
	contentClassName?: string;
	footerClassName?: string;
}>({
	showHeader: true,
	showFooter: true,
	contentClassName: "",
	footerClassName: "",
});

export const FrameConfigProvider = FrameConfigContext.Provider;

export const Header = (props: React.ComponentProps<typeof DialogHeader>) => {
	const { showHeader } = useContext(FrameConfigContext);
	const isInModal = useContext(IsInModalContext);
	if (!showHeader) {
		return null;
	}
	return isInModal ? <DialogHeader {...props} /> : <CardHeader {...props} />;
};

export const Title = (props: React.ComponentProps<typeof DialogTitle>) => {
	const isInModal = useContext(IsInModalContext);
	return isInModal ? <DialogTitle {...props} /> : <CardTitle {...props} />;
};

export const Description = (
	props: React.HTMLAttributes<HTMLParagraphElement>,
) => {
	const isInModal = useContext(IsInModalContext);
	return isInModal ? (
		<DialogDescription {...props} />
	) : (
		<CardDescription {...props} />
	);
};

export const Content = (
	props: React.HTMLAttributes<HTMLDivElement> & {
		ref?: React.Ref<HTMLDivElement>;
	},
) => {
	const { contentClassName } = useContext(FrameConfigContext);
	const isInModal = useContext(IsInModalContext);
	return isInModal ? (
		<div
			{...props}
			className={cn(
				"flex-1 min-w-0 max-w-full",
				props.className,
				contentClassName,
			)}
		/>
	) : (
		<CardContent
			{...props}
			className={cn(props.className, contentClassName)}
		/>
	);
};

export const Footer = (props: React.ComponentProps<typeof DialogFooter>) => {
	const { showFooter, footerClassName } = useContext(FrameConfigContext);
	const isInModal = useContext(IsInModalContext);
	if (showFooter === false) {
		return null;
	}
	return isInModal ? (
		<DialogFooter
			{...props}
			className={cn(props.className, footerClassName)}
		/>
	) : (
		<CardFooter
			{...props}
			className={cn(props.className, footerClassName)}
		/>
	);
};
