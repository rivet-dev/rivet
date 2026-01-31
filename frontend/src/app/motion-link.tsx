import { createLink } from "@tanstack/react-router";
import { motion } from "framer-motion";
import React, { type ComponentProps } from "react";

const MotionLinkComponent = React.forwardRef<
	HTMLAnchorElement,
	ComponentProps<typeof motion.a>
>((props, ref) => {
	return <motion.a ref={ref} {...props} />;
});

export const MotionLink = createLink(MotionLinkComponent);
