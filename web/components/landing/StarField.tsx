"use client";

import React, { useEffect, useRef } from "react";

class Star {
  x: number;
  y: number;
  radius: number;
  opacity: number;
  speed: number;
  twinkleSpeed: number;
  twinkleOffset: number;

  constructor(width: number, height: number) {
    this.x = Math.random() * width;
    this.y = Math.random() * height;
    this.radius = Math.random() * 1.5 + 0.5;
    this.opacity = Math.random() * 0.7 + 0.2;
    this.speed = Math.random() * 0.05 + 0.01;
    this.twinkleSpeed = Math.random() * 0.05 + 0.01;
    this.twinkleOffset = Math.random() * Math.PI * 2;
  }

  draw(context: CanvasRenderingContext2D, width: number, height: number, time: number) {
    const currentOpacity = this.opacity * (0.7 + 0.3 * Math.sin(time * this.twinkleSpeed + this.twinkleOffset));
    
    context.beginPath();
    context.arc(this.x, this.y, this.radius, 0, Math.PI * 2);
    context.fillStyle = `rgba(232, 237, 245, ${currentOpacity})`;
    context.fill();

    // Parallax drift
    this.y += this.speed;
    if (this.y > height) {
      this.y = 0;
      this.x = Math.random() * width;
    }
  }
}

export const StarField = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animationFrameIdValue: number;
    let stars: Star[] = [];

    const init = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
      stars = Array.from({ length: 200 }, () => new Star(canvas.width, canvas.height));
    };

    const animate = (time: number) => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      stars.forEach((star) => star.draw(ctx, canvas.width, canvas.height, time * 0.1));
      animationFrameIdValue = requestAnimationFrame(animate);
    };

    init();
    animate(0);

    const handleResize = () => {
      init();
    };

    window.addEventListener("resize", handleResize);

    return () => {
      cancelAnimationFrame(animationFrameIdValue);
      window.removeEventListener("resize", handleResize);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="fixed inset-0 z-0 pointer-events-none"
      style={{ background: "transparent" }}
    />
  );
};
