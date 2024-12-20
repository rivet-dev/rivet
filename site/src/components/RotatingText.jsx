import React, { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

const RotatingText = ({ texts, interval = 1400 }) => {
  const [index, setIndex] = useState(0);
  const containerRef = useRef(null);
  const textRef = useRef(null);
  const measureRef = useRef(null);
  const [width, setWidth] = useState(0);
  const [height, setHeight] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setIndex((prevIndex) => (prevIndex + 1) % texts.length);
    }, interval);

    return () => clearInterval(timer);
  }, [texts, interval]);

  useEffect(() => {
    if (measureRef.current) {
      measureRef.current.textContent = texts[index];
      const newWidth = measureRef.current.offsetWidth;
      const newHeight = measureRef.current.offsetHeight;
      setWidth(newWidth);
      setHeight(newHeight);
    }
  }, [texts[index]]);

  return (
    <>
      <motion.span
        ref={containerRef}
        style={{ 
          display: 'inline-block', 
          position: 'relative', 
          overflow: 'visible',
          height: height ? `${height}px` : 'auto',
          verticalAlign: 'bottom'
        }}
        animate={{ width }}
        transition={{ duration: 0.6 }}
      >
        <AnimatePresence mode="wait">
          <motion.span
            ref={textRef}
            key={index}
            initial={{ y: 20, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: -20, opacity: 0 }}
            transition={{ duration: 0.3 }}
            style={{ position: 'absolute', whiteSpace: 'nowrap', left: 0, bottom: 0 }}
          >
            {texts[index]}
          </motion.span>
        </AnimatePresence>
      </motion.span>
      <span 
        ref={measureRef} 
        style={{ 
          visibility: 'hidden', 
          position: 'absolute', 
          whiteSpace: 'nowrap',
          pointerEvents: 'none'
        }}
      />
    </>
  );
};

export default RotatingText;