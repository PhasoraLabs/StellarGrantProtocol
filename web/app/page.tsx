import React from "react";
import { StarField } from "@/components/landing/StarField";
import { HeroSection } from "@/components/landing/HeroSection";
import { LiveStatsBar } from "@/components/landing/LiveStatsBar";
import { FeatureCards } from "@/components/landing/FeatureCards";
import { HowItWorks } from "@/components/landing/HowItWorks";
import { SocialProof } from "@/components/landing/SocialProof";
import { CtaBanner } from "@/components/landing/CtaBanner";
import { Footer } from "@/components/landing/Footer";

export default function LandingPage() {
  return (
    <main className="min-h-screen bg-bg-primary text-text-primary selection:bg-accent-primary/30 selection:text-accent-primary">
      {/* Fixed background layer */}
      <StarField />
      
      {/* Content layer */}
      <div className="relative z-10">
        <HeroSection />
        <LiveStatsBar />
        <FeatureCards />
        <HowItWorks />
        <SocialProof />
        <CtaBanner />
        <Footer />
      </div>
    </main>
  );
}
