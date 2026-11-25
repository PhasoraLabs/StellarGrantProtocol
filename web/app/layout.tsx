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
    title: "StellarGrant Protocol",
    description: "Decentralized milestone-based grant management on Stellar",
    type: "website",
  },
  twitter: {
    card: "summary",
    title: "StellarGrant Protocol",
    description: "Decentralized milestone-based grant management on Stellar",
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
        className={`${orbitron.variable} ${ibmPlexMono.variable} antialiased [&_a:focus-visible]:outline-2 [&_a:focus-visible]:outline-offset-2 [&_a:focus-visible]:outline-accent-primary [&_button:focus-visible]:outline-2 [&_button:focus-visible]:outline-offset-2 [&_button:focus-visible]:outline-accent-primary [&_select:focus-visible]:outline-2 [&_select:focus-visible]:outline-offset-2 [&_select:focus-visible]:outline-accent-primary [&_textarea:focus-visible]:outline-2 [&_textarea:focus-visible]:outline-offset-2 [&_textarea:focus-visible]:outline-accent-primary [&_input:not(.sr-only):focus-visible]:outline-2 [&_input:not(.sr-only):focus-visible]:outline-offset-2 [&_input:not(.sr-only):focus-visible]:outline-accent-primary`}
      >
        <a
          href="#main-content"
          className="sr-only focus:not-sr-only focus:fixed focus:top-4 focus:left-4 focus:z-[100] focus:bg-accent-primary focus:text-bg-primary focus:px-4 focus:py-2 focus:font-mono focus:text-xs focus:font-bold focus:uppercase focus:tracking-wider focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-secondary"
        >
          Skip to main content
        </a>
        <Providers>
          <AppHeader />
          <main
            id="main-content"
            tabIndex={-1}
            className="min-h-[calc(100vh-64px)] relative z-10 focus:outline-none"
          >
            {children}
          </main>
          <AppFooter />
        </Providers>
      </body>
    </html>
  );
}
