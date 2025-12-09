"use client";

import { cn } from "@/lib/utils";
import { Children, useCallback, useEffect, useRef, useState } from "react";

const useAutoScroll = (
  containerRef: React.RefObject<HTMLDivElement | null>,
  enabled: boolean,
) => {
  const [autoScrollEnabled, setAutoScrollEnabled] = useState(true);
  const lastScrollTopRef = useRef(0);
  const autoScrollingRef = useRef(false);
  const [newMessageAdded, setNewMessageAdded] = useState(false);
  const prevChildrenCountRef = useRef(0);
  const scrollTriggeredRef = useRef(false);

  const isAtBottom = useCallback((element: HTMLDivElement) => {
    const { scrollTop, scrollHeight, clientHeight } = element;
    return scrollHeight - scrollTop - clientHeight <= 8;
  }, []);

  const scrollToBottom = useCallback(
    (behavior: ScrollBehavior = "instant") => {
      const container = containerRef.current;
      if (!container) return;

      autoScrollingRef.current = true;
      scrollTriggeredRef.current = true;

      const targetScrollTop = container.scrollHeight - container.clientHeight;

      container.scrollTo({
        top: targetScrollTop,
        behavior: behavior,
      });

      const checkScrollEnd = () => {
        if (Math.abs(container.scrollTop - targetScrollTop) < 5) {
          autoScrollingRef.current = false;
          scrollTriggeredRef.current = false;
          return;
        }

        requestAnimationFrame(checkScrollEnd);
      };

      requestAnimationFrame(checkScrollEnd);

      const safetyTimeout = setTimeout(() => {
        autoScrollingRef.current = false;
        scrollTriggeredRef.current = false;
      }, 500);

      try {
        const handleScrollEnd = () => {
          autoScrollingRef.current = false;
          scrollTriggeredRef.current = false;
          clearTimeout(safetyTimeout);
          container.removeEventListener("scrollend", handleScrollEnd);
        };

        container.addEventListener("scrollend", handleScrollEnd, {
          once: true,
        });
      } catch (e) {
        // scrollend event not supported in this browser, fallback to requestAnimationFrame
      }
    },
    [containerRef],
  );

  useEffect(() => {
    if (!enabled) return;

    const container = containerRef?.current;
    if (!container) return;

    lastScrollTopRef.current = container.scrollTop;

    const handleScroll = () => {
      if (autoScrollingRef.current) return;

      const currentScrollTop = container.scrollTop;

      if (currentScrollTop < lastScrollTopRef.current && autoScrollEnabled) {
        setAutoScrollEnabled(false);
      }

      if (isAtBottom(container) && !autoScrollEnabled) {
        setAutoScrollEnabled(true);
      }

      lastScrollTopRef.current = currentScrollTop;
    };

    const handleWheel = (e: WheelEvent) => {
      if (e.deltaY < 0 && autoScrollEnabled) {
        setAutoScrollEnabled(false);
      }
    };

    const handleTouchStart = () => {
      lastScrollTopRef.current = container.scrollTop;
    };

    const handleTouchMove = () => {
      if (container.scrollTop < lastScrollTopRef.current && autoScrollEnabled) {
        setAutoScrollEnabled(false);
      }

      lastScrollTopRef.current = container.scrollTop;
    };

    const handleTouchEnd = () => {
      if (isAtBottom(container) && !autoScrollEnabled) {
        setAutoScrollEnabled(true);
      }
    };

    container.addEventListener("scroll", handleScroll, { passive: true });
    container.addEventListener("wheel", handleWheel, { passive: true });
    container.addEventListener("touchstart", handleTouchStart, {
      passive: true,
    });
    container.addEventListener("touchmove", handleTouchMove, { passive: true });
    container.addEventListener("touchend", handleTouchEnd, { passive: true });

    return () => {
      container.removeEventListener("scroll", handleScroll);
      container.removeEventListener("wheel", handleWheel);
      container.removeEventListener("touchstart", handleTouchStart);
      container.removeEventListener("touchmove", handleTouchMove);
      container.removeEventListener("touchend", handleTouchEnd);
    };
  }, [containerRef, enabled, autoScrollEnabled, isAtBottom]);

  return {
    autoScrollEnabled,
    scrollToBottom,
    isScrolling: autoScrollingRef.current,
    scrollTriggered: scrollTriggeredRef.current,
    newMessageAdded,
    setNewMessageAdded,
    prevChildrenCountRef,
  };
};

export type ChatContainerProps = {
  children: React.ReactNode;
  className?: string;
  autoScroll?: boolean;
  scrollToRef?: React.RefObject<HTMLDivElement | null>;
  ref?: React.RefObject<HTMLDivElement | null>;
} & React.HTMLAttributes<HTMLDivElement>;

function ChatContainer({
  className,
  children,
  autoScroll = true,
  scrollToRef,
  ref,
  ...props
}: ChatContainerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const localBottomRef = useRef<HTMLDivElement>(null);
  const bottomRef = scrollToRef || localBottomRef;
  const chatContainerRef = ref || containerRef;
  const prevChildrenRef = useRef<React.ReactNode>(null);
  const contentChangedWithoutNewMessageRef = useRef(false);

  const {
    autoScrollEnabled,
    scrollToBottom,
    isScrolling,
    scrollTriggered,
    newMessageAdded,
    setNewMessageAdded,
    prevChildrenCountRef,
  } = useAutoScroll(chatContainerRef, autoScroll);

  useEffect(() => {
    const childrenArray = Children.toArray(children);
    const currentChildrenCount = childrenArray.length;

    if (currentChildrenCount > prevChildrenCountRef.current) {
      setNewMessageAdded(true);
    } else if (prevChildrenRef.current !== children) {
      contentChangedWithoutNewMessageRef.current = true;
    }

    prevChildrenCountRef.current = currentChildrenCount;
    prevChildrenRef.current = children;
  }, [children, setNewMessageAdded]);

  useEffect(() => {
    if (!autoScroll) return;

    const scrollHandler = () => {
      if (newMessageAdded) {
        scrollToBottom("instant");
        setNewMessageAdded(false);
        contentChangedWithoutNewMessageRef.current = false;
      } else if (
        contentChangedWithoutNewMessageRef.current &&
        autoScrollEnabled &&
        !isScrolling &&
        !scrollTriggered
      ) {
        scrollToBottom("instant");
        contentChangedWithoutNewMessageRef.current = false;
      }
    };

    requestAnimationFrame(scrollHandler);
  }, [
    children,
    autoScroll,
    autoScrollEnabled,
    isScrolling,
    scrollTriggered,
    scrollToBottom,
    newMessageAdded,
    setNewMessageAdded,
  ]);

  return (
    <div
      className={cn(
        "flex flex-col overflow-y-auto h-full p-3", // Add more padding to accommodate code block
        className,
      )}
      role="log"
      ref={chatContainerRef}
      {...props}
    >
      {children}
      <div
        ref={bottomRef}
        className="h-[1px] w-full flex-shrink-0 scroll-mt-4"
        aria-hidden="true"
      />
    </div>
  );
}

export { ChatContainer };
