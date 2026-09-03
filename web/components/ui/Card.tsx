"use client";

import React from "react";
import { motion } from "framer-motion";

interface CardProps {
  children: React.ReactNode;
  className?: string;
}

export const Card = ({ children, className = "" }: CardProps) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      whileHover={{ 
        boxShadow: "0 0 20px rgba(59, 130, 246, 0.15), 0 0 1px rgba(59, 130, 246, 0.4)",
        borderColor: "rgba(59, 130, 246, 0.5)"
      }}
      transition={{ duration: 0.3, ease: "easeOut" }}
      className={`bg-surface border border-border-color rounded-[4px] p-6 transition-colors duration-300 ${className}`}
    >
      {children}
    </motion.div>
  );
};
