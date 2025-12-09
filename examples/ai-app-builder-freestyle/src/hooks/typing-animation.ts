import { useEffect, useState } from "react";

interface UseTypingAnimationOptions {
  texts: string[];
  baseText?: string;
  typingSpeed?: number;
  erasingSpeed?: number;
  pauseDuration?: number;
  initialDelay?: number;
}

export function useTypingAnimation({
  texts,
  baseText = "",
  typingSpeed = 100,
  erasingSpeed = 50,
  pauseDuration = 2000,
  initialDelay = 500,
}: UseTypingAnimationOptions) {
  const [isMounted, setIsMounted] = useState(false);
  const [displayText, setDisplayText] = useState(baseText);

  useEffect(() => {
    setIsMounted(true);
  }, []);

  useEffect(() => {
    if (!isMounted || texts.length === 0) return;

    let currentTextIndex = 0;
    let currentCharIndex = 0;
    let typingTimer: NodeJS.Timeout;
    let pauseTimer: NodeJS.Timeout;

    const typeNextCharacter = () => {
      const currentText = texts[currentTextIndex];
      if (currentCharIndex < currentText.length) {
        const newText =
          baseText +
          (baseText ? " " : "") +
          currentText.substring(0, currentCharIndex + 1);
        setDisplayText(newText);
        currentCharIndex++;
        typingTimer = setTimeout(typeNextCharacter, typingSpeed);
      } else {
        pauseTimer = setTimeout(() => {
          eraseText();
        }, pauseDuration);
      }
    };

    const eraseText = () => {
      const currentText = texts[currentTextIndex];
      if (currentCharIndex > 0) {
        const newText =
          baseText +
          (baseText ? " " : "") +
          currentText.substring(0, currentCharIndex - 1);
        setDisplayText(newText);
        currentCharIndex--;
        typingTimer = setTimeout(eraseText, erasingSpeed);
      } else {
        // Move to the next text
        currentTextIndex = (currentTextIndex + 1) % texts.length;
        pauseTimer = setTimeout(() => {
          typingTimer = setTimeout(typeNextCharacter, typingSpeed);
        }, 500);
      }
    };

    typingTimer = setTimeout(typeNextCharacter, initialDelay);

    return () => {
      clearTimeout(typingTimer);
      clearTimeout(pauseTimer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isMounted]); // Only depend on isMounted to avoid re-running

  return {
    displayText,
    isAnimating: isMounted,
  };
}
