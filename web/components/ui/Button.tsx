"use client";

import React from "react";
import { motion } from "framer-motion";
import Link from "next/link";

const MotionLink = motion(Link);

interface ButtonProps {
  variant?: "primary" | "ghost";
  children: React.ReactNode;
  href?: string;
  onClick?: () => void;
  className?: string;
  disabled?: boolean;
}

export const Button = ({
  variant = "primary",
  children,
  href,
  onClick,
  className = "",
  disabled = false,
}: ButtonProps) => {
  const baseStyles = "inline-flex items-center justify-center px-8 py-3 font-orbitron text-sm font-bold transition-all duration-300 rounded-none border-0 uppercase tracking-wider";
  
  const variants = {
    primary: "bg-accent-primary text-bg-primary hover:bg-opacity-90",
    ghost: "bg-transparent border border-accent-primary text-accent-primary hover:bg-accent-primary hover:text-bg-primary",
  };

  const combinedClassName = `${baseStyles} ${variants[variant]} ${className}`;

  if (href) {
    return (
      <MotionLink 
        href={href} 
        className={combinedClassName}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
      >
        {children}
      </MotionLink>
    );
  }

  return (
    <motion.button 
      onClick={onClick} 
      className={combinedClassName}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      disabled={disabled}
    >
      {children}
    </motion.button>
  );
};
