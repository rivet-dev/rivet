'use client';

import { useEffect, useRef, useState } from 'react';

export function HeroBackground() {
  // Configuration object for all particle behavior constants
  const PARTICLE_CONFIG = {
    // Particle appearance
    CIRCLE_RADIUS: 3,
    MAX_OPACITY: 0.05,
    ORANGE_COLOR: '#ff6b35', // Brighter orange

    // Grid layout
    GRID_SPACING: 40,

    // Mouse interaction
    PATH_FORCE_RADIUS: 200, // Radius for path-based forces
    BASE_PUSH_FORCE: 0.3,
    MOVEMENT_FORCE_MULTIPLIER: 4.0,
    MAX_MOUSE_VELOCITY: 5000,

    // Physics
    DAMPING: 0.975,
    SPRING_STRENGTH: 0.05,

    // Animation
    TARGET_FPS: 60,
    MAX_FRAME_TIME: 0.1, // seconds

    // Pulse effect
    PULSE_SPEED: 150, // pixels per second - faster wave
    PULSE_PEAK_OPACITY: 0.15, // less opaque
    PULSE_LOOP_INTERVAL: 8000, // milliseconds between pulses - less frequent
    PULSE_DURATION: 1500, // how long each pulse lasts at a particle

    // Intro pulse effect
    INTRO_PULSE_SPEED: 1200, // pixels per second - extremely fast

    // Click pulse effect
    CLICK_PULSE_SPEED: 1600, // pixels per second - 2x faster
    CLICK_PULSE_PEAK_OPACITY: 0.2, // slightly more visible than regular pulses
    CLICK_PULSE_DURATION: 800, // shorter duration for snappy effect

    // Debug
    DEBUG_MOUSE_TRACKING: false,
    DEBUG_DOT_RADIUS: 2,
    DEBUG_MAX_DOTS: 100,
    DEBUG_EXCLUSION_ZONES: false // Set to true to visualize exclusion zones
  };

  // Simple Particle class for grid-based circles
  class Particle {
    x: number;
    y: number;
    baseOpacity: number;
    hasBeenRevealed: boolean;
    distanceFromCenter: number;
    pulseOpacity: number;
    mouseX: number;
    mouseY: number;

    constructor(x: number, y: number) {
      // Fixed position
      this.x = x;
      this.y = y;

      // Visual properties
      this.baseOpacity = 0; // Start at 0, will be set by intro pulse
      this.hasBeenRevealed = false; // Track if intro pulse has reached this particle

      // Pulse effect properties
      this.distanceFromCenter = 0;
      this.pulseOpacity = 0;
      this.mouseX = 0;
      this.mouseY = 0;
    }

    draw(ctx: CanvasRenderingContext2D, config: typeof PARTICLE_CONFIG) {
      // Calculate total opacity: base + pulse
      let totalOpacity = this.baseOpacity + this.pulseOpacity;

      // Add subtle mouse proximity highlighting
      const mouseDistance = Math.sqrt((this.x - this.mouseX) ** 2 + (this.y - this.mouseY) ** 2);
      if (mouseDistance < config.PATH_FORCE_RADIUS) {
        const proximityFactor = 1 - mouseDistance / config.PATH_FORCE_RADIUS;
        totalOpacity += proximityFactor * 0.07; // Slightly less intense highlight
      }

      if (totalOpacity <= 0) return;

      ctx.save();
      ctx.globalAlpha = Math.min(1, totalOpacity);
      ctx.fillStyle = 'white';
      ctx.beginPath();
      ctx.arc(this.x, this.y, config.CIRCLE_RADIUS, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    update(activePulses: any[], mouseX: number, mouseY: number, config: typeof PARTICLE_CONFIG) {
      this.mouseX = mouseX;
      this.mouseY = mouseY;
      this.updatePulse(activePulses, config);
    }

    updatePulse(activePulses: any[], config: typeof PARTICLE_CONFIG) {
      const currentTime = performance.now();
      this.pulseOpacity = 0;

      // Check all active pulses
      activePulses.forEach(pulse => {
        const timeSincePulseStart = currentTime - pulse.startTime;

        // Use different speed for different pulse types
        let pulseSpeed, pulseDuration, peakOpacity;
        if (pulse.isIntro) {
          pulseSpeed = config.INTRO_PULSE_SPEED;
          pulseDuration = config.PULSE_DURATION;
          peakOpacity = config.PULSE_PEAK_OPACITY;
        } else if (pulse.isClick) {
          pulseSpeed = config.CLICK_PULSE_SPEED;
          pulseDuration = config.CLICK_PULSE_DURATION;
          peakOpacity = config.CLICK_PULSE_PEAK_OPACITY;
        } else {
          pulseSpeed = config.PULSE_SPEED;
          pulseDuration = config.PULSE_DURATION;
          peakOpacity = config.PULSE_PEAK_OPACITY;
        }

        // Calculate when this pulse wave should reach this particle
        const waveRadius = (timeSincePulseStart / 1000) * pulseSpeed;

        // Calculate distance from this pulse's center (not always canvas center)
        const particleDistance = Math.sqrt((this.x - pulse.centerX) ** 2 + (this.y - pulse.centerY) ** 2);

        // Check if the wave has reached this particle
        if (waveRadius >= particleDistance) {
          // For intro pulse, permanently set base opacity when wave reaches particle
          if (pulse.isIntro && !this.hasBeenRevealed) {
            this.baseOpacity = config.MAX_OPACITY;
            this.hasBeenRevealed = true;
          }

          // Calculate how long the wave has been at this particle
          const timeAtParticle = timeSincePulseStart - (particleDistance / pulseSpeed) * 1000;

          if (timeAtParticle >= 0 && timeAtParticle < pulseDuration) {
            const pulsePhase = timeAtParticle / pulseDuration;

            let pulseContribution = 0;

            if (pulse.isIntro) {
              // Intro pulse: starts at 0, peaks, then settles at base opacity
              if (pulsePhase < 0.3) {
                // Rising phase: 0 to peak
                const riseProgress = pulsePhase / 0.3;
                pulseContribution = riseProgress * peakOpacity;
              } else {
                // Falling phase: peak to 0 (base opacity already set above)
                const fallProgress = (pulsePhase - 0.3) / 0.7;
                pulseContribution = peakOpacity * (1 - fallProgress);
              }
            } else {
              // Regular and click pulses: normal pulse behavior
              if (pulsePhase < 0.3) {
                // Rising phase: 0 to peak
                pulseContribution = (pulsePhase / 0.3) * peakOpacity;
              } else if (pulsePhase < 1) {
                // Falling phase: peak to 0
                const fallProgress = (pulsePhase - 0.3) / 0.7;
                pulseContribution = peakOpacity * (1 - fallProgress);
              }
            }

            // Add this pulse's contribution (multiple pulses can stack)
            this.pulseOpacity += pulseContribution;
          }
        }
      });

      // Clamp to maximum opacity
      this.pulseOpacity = Math.min(this.pulseOpacity, config.PULSE_PEAK_OPACITY * 2);
    }
  }

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);
  const [particles, setParticles] = useState<Particle[]>([]);
  const [activePulses, setActivePulses] = useState<any[]>([]);
  const [mousePosition, setMousePosition] = useState({ x: 0, y: 0 });
  const [systemStartTime, setSystemStartTime] = useState(0);
  const [isVisible, setIsVisible] = useState(false);
  const animationFrameRef = useRef<number | null>(null);
  const lastFrameTimeRef = useRef(0);

  // Initialize the particle system
  useEffect(() => {
    if (typeof window !== 'undefined') {
      setIsVisible(window.innerWidth >= 750);
    }

    const canvas = canvasRef.current;
    const wrapper = wrapperRef.current;
    if (!canvas || !wrapper) return;

    // Get landing elements for positioning
    const landingHero =
      document.querySelector('.landing-hero') ||
      document.querySelector('.hero-section') ||
      document.querySelector('[data-hero-section]');

    // Calculate container height
    let containerHeight = 600; // Fixed height to match HeroSection

    if (landingHero) {
      containerHeight = landingHero.offsetHeight;
    }

    // Set canvas size to match full screen (excluding scrollbar)
    canvas.width = typeof window !== 'undefined' ? document.documentElement.clientWidth : 800;
    canvas.height = containerHeight;

    // Create grid-based particles
    const newParticles = createGridParticles(canvas, 0, 0);
    setParticles(newParticles);

    // Initialize timing
    const startTime = performance.now();
    setSystemStartTime(startTime);

    // Add immediate intro pulse that moves very fast
    setActivePulses([
      {
        startTime: startTime,
        centerX: canvas.width / 2,
        centerY: canvas.height / 2,
        isIntro: true // Mark as intro pulse for different speed
      }
    ]);

    // Initialize mouse position to center of canvas
    setMousePosition({ x: canvas.width / 2, y: canvas.height / 2 });

    lastFrameTimeRef.current = startTime;

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, []);

  // Create particles in a grid layout
  const createGridParticles = (
    canvas: HTMLCanvasElement,
    leftPadding: number,
    rightPadding: number
  ): Particle[] => {
    if (!canvas) return [];

    const newParticles: Particle[] = [];
    const baseSpacing = PARTICLE_CONFIG.GRID_SPACING;
    const edgeMargin = 40; // Constant distance from edges

    // Get exclusion zones from landing-hero direct children
    const exclusionZones = getExclusionZones(wrapperRef.current);

    // Calculate available space after accounting for edge margins
    const availableWidth = canvas.width - 2 * edgeMargin;
    const availableHeight = canvas.height - 2 * edgeMargin;

    // Calculate maximum number of particles that fit with base spacing
    const maxCols = Math.floor(availableWidth / baseSpacing);
    const maxRows = Math.floor(availableHeight / baseSpacing);

    // Adjust spacing to distribute particles evenly with exact edge margins
    const actualSpacingX = maxCols > 0 ? availableWidth / maxCols : baseSpacing;
    const actualSpacingY = maxRows > 0 ? availableHeight / maxRows : baseSpacing;

    // Calculate center for opacity calculations
    const centerX = canvas.width / 2;
    const centerY = canvas.height / 2;

    for (let row = 0; row <= maxRows; row++) {
      for (let col = 0; col <= maxCols; col++) {
        const x = edgeMargin + col * actualSpacingX;
        const y = edgeMargin + row * actualSpacingY;

        // Skip if particle overlaps with any exclusion zone
        if (isPointInExclusionZones(x, y, exclusionZones)) continue;

        const particle = new Particle(x, y);

        // Calculate distance from center for pulse effect
        particle.distanceFromCenter = Math.sqrt((x - centerX) ** 2 + (y - centerY) ** 2);

        // Start with zero opacity - intro pulse will reveal them
        particle.baseOpacity = 0;

        newParticles.push(particle);
      }
    }

    return newParticles;
  };

  // Get bounding rectangles of elements marked for exclusion
  const getExclusionZones = (wrapper: HTMLDivElement | null) => {
    if (!wrapper) return [];

    const exclusionZones: any[] = [];
    const margin = 25;

    // Find all elements with the exclusion class
    const elements = document.querySelectorAll('.hero-bg-exclude');

    elements.forEach(element => {
      const rect = element.getBoundingClientRect();

      // Get canvas positioning relative to viewport
      const canvas = canvasRef.current;
      if (!canvas) return;

      // Get the actual canvas position in the viewport
      const canvasRect = canvas.getBoundingClientRect();

      // Convert to canvas coordinates (accounting for canvas position) and add margin
      const exclusionRect = {
        left: rect.left - canvasRect.left - margin,
        top: rect.top - canvasRect.top - margin,
        right: rect.right - canvasRect.left + margin,
        bottom: rect.bottom - canvasRect.top + margin
      };

      // Only add if the rect intersects with canvas bounds
      if (
        exclusionRect.right > 0 &&
        exclusionRect.left < canvas.width &&
        exclusionRect.bottom > 0 &&
        exclusionRect.top < canvas.height
      ) {
        exclusionZones.push(exclusionRect);
      }
    });

    return exclusionZones;
  };

  // Check if a point (with circle radius) overlaps with any exclusion zone
  const isPointInExclusionZones = (x: number, y: number, exclusionZones: any[]) => {
    const radius = PARTICLE_CONFIG.CIRCLE_RADIUS;

    for (const zone of exclusionZones) {
      // Check if circle centered at (x, y) with given radius overlaps with rectangle
      const closestX = Math.max(zone.left, Math.min(x, zone.right));
      const closestY = Math.max(zone.top, Math.min(y, zone.bottom));

      const distanceX = x - closestX;
      const distanceY = y - closestY;
      const distanceSq = distanceX * distanceX + distanceY * distanceY;

      if (distanceSq <= radius * radius) {
        return true;
      }
    }

    return false;
  };

  // Draw exclusion zones for debugging
  const drawExclusionZones = (ctx: CanvasRenderingContext2D) => {
    const wrapper = wrapperRef.current;
    if (!wrapper) return;

    const exclusionZones = getExclusionZones(wrapper);

    ctx.save();
    ctx.strokeStyle = 'red';
    ctx.fillStyle = 'rgba(255, 0, 0, 0.2)';
    ctx.lineWidth = 2;

    exclusionZones.forEach(zone => {
      // Draw filled rectangle
      ctx.fillRect(zone.left, zone.top, zone.right - zone.left, zone.bottom - zone.top);
      // Draw border
      ctx.strokeRect(zone.left, zone.top, zone.right - zone.left, zone.bottom - zone.top);
    });

    ctx.restore();
  };

  // Manage pulse creation and cleanup
  const managePulses = (currentTime: number) => {
    const timeSinceSystemStart = currentTime - systemStartTime;

    // Calculate how many regular pulses we should have (excluding the intro pulse)
    // Start regular pulses after a delay to let the intro pulse finish
    const regularPulseStartDelay = 2000; // 2 seconds after system start
    if (timeSinceSystemStart < regularPulseStartDelay) {
      return;
    }

    const adjustedTime = timeSinceSystemStart - regularPulseStartDelay;
    const shouldHaveRegularPulseCount = Math.floor(adjustedTime / PARTICLE_CONFIG.PULSE_LOOP_INTERVAL) + 1;
    const currentRegularPulseCount = activePulses.filter(p => !p.isIntro).length;

    // Create new regular pulses if needed
    if (currentRegularPulseCount < shouldHaveRegularPulseCount) {
      const regularPulseIndex = currentRegularPulseCount;
      const pulseStartTime =
        systemStartTime + regularPulseStartDelay + regularPulseIndex * PARTICLE_CONFIG.PULSE_LOOP_INTERVAL;

      const canvas = canvasRef.current;
      setActivePulses(prev => [
        ...prev,
        {
          startTime: pulseStartTime,
          centerX: canvas ? canvas.width / 2 : 0,
          centerY: canvas ? canvas.height / 2 : 0,
          isIntro: false
        }
      ]);
    }

    // Clean up pulses that are too old (beyond their effective range)
    const canvas = canvasRef.current;
    const maxCanvasDiagonal = canvas ? Math.sqrt(canvas.width ** 2 + canvas.height ** 2) : 1000;

    setActivePulses(prev =>
      prev.filter(pulse => {
        let pulseSpeed, pulseDuration;
        if (pulse.isIntro) {
          pulseSpeed = PARTICLE_CONFIG.INTRO_PULSE_SPEED;
          pulseDuration = PARTICLE_CONFIG.PULSE_DURATION;
        } else if (pulse.isClick) {
          pulseSpeed = PARTICLE_CONFIG.CLICK_PULSE_SPEED;
          pulseDuration = PARTICLE_CONFIG.CLICK_PULSE_DURATION;
        } else {
          pulseSpeed = PARTICLE_CONFIG.PULSE_SPEED;
          pulseDuration = PARTICLE_CONFIG.PULSE_DURATION;
        }

        const maxPulseLifetime = (maxCanvasDiagonal / pulseSpeed) * 1000 + pulseDuration;
        return currentTime - pulse.startTime < maxPulseLifetime;
      })
    );
  };

  // Animation loop
  useEffect(() => {
    const TARGET_FPS = PARTICLE_CONFIG.TARGET_FPS;
    const FRAME_TIME = 1000 / TARGET_FPS;

    const animate = () => {
      const currentTime = performance.now();
      const timeSinceLastFrame = currentTime - lastFrameTimeRef.current;

      // Only render if enough time has passed for next frame
      if (timeSinceLastFrame >= FRAME_TIME) {
        lastFrameTimeRef.current = currentTime;

        // Manage pulse creation and cleanup
        managePulses(currentTime);

        // Update particles
        particles.forEach(particle => {
          particle.update(activePulses, mousePosition.x, mousePosition.y, PARTICLE_CONFIG);
        });

        // Draw particles
        const canvas = canvasRef.current;
        const ctx = canvas?.getContext('2d');
        if (canvas && ctx) {
          // Clear canvas
          ctx.clearRect(0, 0, canvas.width, canvas.height);

          // Draw particles
          particles.forEach(particle => {
            particle.draw(ctx, PARTICLE_CONFIG);
          });

          // Draw exclusion zones if debug mode is enabled
          if (PARTICLE_CONFIG.DEBUG_EXCLUSION_ZONES) {
            drawExclusionZones(ctx);
          }
        }
      }

      // Continue animation
      animationFrameRef.current = requestAnimationFrame(animate);
    };

    if (particles.length > 0) {
      animationFrameRef.current = requestAnimationFrame(animate);
    }

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [particles, activePulses, mousePosition, systemStartTime]);

  // Handle mouse movement
  const handleMouseMove = (e: MouseEvent) => {
    if (wrapperRef.current) {
      const wrapperRect = wrapperRef.current.getBoundingClientRect();
      setMousePosition({
        x: e.clientX - wrapperRect.left,
        y: e.clientY - wrapperRect.top
      });
    }
  };

  // Handle mouse clicks to create pulses
  const handleMouseDown = (e: MouseEvent) => {
    let clickX, clickY;
    if (wrapperRef.current) {
      const wrapperRect = wrapperRef.current.getBoundingClientRect();
      clickX = e.clientX - wrapperRect.left;
      clickY = e.clientY - wrapperRect.top;
    } else {
      clickX = e.pageX;
      clickY = e.pageY;
    }

    // Create a new click pulse at the mousedown position
    setActivePulses(prev => [
      ...prev,
      {
        startTime: performance.now(),
        centerX: clickX,
        centerY: clickY,
        isClick: true // Mark as click pulse for different behavior
      }
    ]);
  };

  // Set up global mouse event listeners
  useEffect(() => {
    if (typeof window !== 'undefined') {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mousedown', handleMouseDown);

      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mousedown', handleMouseDown);
      };
    }
  }, []);

  // Handle resize
  useEffect(() => {
    const handleResize = () => {
      const canvas = canvasRef.current;
      const wrapper = wrapperRef.current;

      // Update visibility based on window width
      if (typeof window !== 'undefined') {
        setIsVisible(window.innerWidth >= 750);
      }

      if (!canvas || !wrapper) return;

      // Get landing elements for positioning
      const landingHero =
        document.querySelector('.landing-hero') ||
        document.querySelector('.hero-section') ||
        document.querySelector('[data-hero-section]');

      // Calculate container height
      let containerHeight = 600; // Fixed height to match HeroSection

      if (landingHero) {
        containerHeight = landingHero.offsetHeight;
      }

      // Set canvas size to match full screen (excluding scrollbar)
      canvas.width = typeof window !== 'undefined' ? document.documentElement.clientWidth : 800;
      canvas.height = containerHeight;

      // Recreate grid particles for new dimensions
      const newParticles = createGridParticles(canvas, 0, 0);
      setParticles(newParticles);

      // Reset mouse position to center of new canvas
      setMousePosition({ x: canvas.width / 2, y: canvas.height / 2 });
    };

    if (typeof window !== 'undefined') {
      window.addEventListener('resize', handleResize);
      // Wait for layout to finish and re-render
      setTimeout(handleResize, 0);

      return () => {
        window.removeEventListener('resize', handleResize);
      };
    }
  }, []);

  // Mutation observer to detect changes to exclusion zone elements
  useEffect(() => {
    if (typeof window === 'undefined') return;

    const regenerateParticles = () => {
      const canvas = canvasRef.current;
      const wrapper = wrapperRef.current;
      if (!canvas || !wrapper) return;

      // Recreate particles with updated exclusion zones
      const newParticles = createGridParticles(canvas, 0, 0);
      setParticles(newParticles);
    };

    // Create mutation observer to watch for DOM changes
    const observer = new MutationObserver(mutations => {
      let shouldRegenerate = false;

      mutations.forEach(mutation => {
        // Check if any exclusion zone elements were added or removed
        if (mutation.type === 'childList') {
          const addedNodes = Array.from(mutation.addedNodes).filter(
            node => node.nodeType === Node.ELEMENT_NODE
          ) as Element[];
          const removedNodes = Array.from(mutation.removedNodes).filter(
            node => node.nodeType === Node.ELEMENT_NODE
          ) as Element[];

          // Check if any added/removed nodes are exclusion zones or contain them
          const hasExclusionChanges = [...addedNodes, ...removedNodes].some(
            node => node.classList?.contains('hero-bg-exclude') || node.querySelector?.('.hero-bg-exclude')
          );

          if (hasExclusionChanges) {
            shouldRegenerate = true;
          }
        }

        // Check if class changes affect exclusion zones
        if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
          const target = mutation.target as Element;
          if (target.classList?.contains('hero-bg-exclude')) {
            shouldRegenerate = true;
          }
        }
      });

      if (shouldRegenerate) {
        // Debounce to avoid excessive regeneration
        setTimeout(regenerateParticles, 100);
      }
    });

    // Observe the entire document for changes
    observer.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['class']
    });

    return () => {
      observer.disconnect();
    };
  }, []);

  return (
    <div
      ref={wrapperRef}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        width: '100%',
        height: '600px',
        overflow: 'hidden',
        pointerEvents: 'none',
        zIndex: -1,
        display: isVisible ? 'block' : 'none'
      }}
    >
      <canvas
        ref={canvasRef}
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: '100%',
          height: '100%',
          pointerEvents: 'none',
          zIndex: -1
        }}
      />
    </div>
  );
}
