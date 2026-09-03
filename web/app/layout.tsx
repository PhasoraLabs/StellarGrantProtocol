import type { Metadata } from "next";
import { Orbitron, IBM_Plex_Mono } from "next/font/google";
import "./globals.css";
import { Providers } from "./providers";
import { AppHeader } from "@/components/layout/AppHeader";
import { AppFooter } from "@/components/layout/AppFooter";

const orbitron = Orbitron({
  variable: "--font-orbitron",
  subsets: ["latin"],
  weight: ["400", "700", "900"],
});

const ibmPlexMono = IBM_Plex_Mono({
  variable: "--font-ibm-plex-mono",
  subsets: ["latin"],
  weight: ["400", "500"],
});

export const metadata: Metadata = {
  title: {
    default: "StellarGrant Protocol",
    template: "%s | StellarGrant",
  },
  description: "Decentralized milestone-based grant management on Stellar",
  openGraph: {
    siteName: "StellarGrant Protocol",
    type: "website",
  },
  other: {
    "theme-color": "#050A14",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body
        className={`${orbitron.variable} ${ibmPlexMono.variable} antialiased`}
      >
        <Providers>
          <AppHeader />
          <main className="min-h-[calc(100vh-64px)] relative z-10">
            {children}
          </main>
          <AppFooter />
        </Providers>
      </body>
    </html>
  );
}
