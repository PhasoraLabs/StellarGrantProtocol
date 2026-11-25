import fs from "node:fs";
import path from "node:path";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

vi.mock("next/font/google", () => ({
  Orbitron: () => ({ variable: "--font-orbitron" }),
  IBM_Plex_Mono: () => ({ variable: "--font-ibm-plex-mono" }),
}));

vi.mock("@/app/providers", () => ({
  Providers: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="providers">{children}</div>
  ),
}));

vi.mock("@/components/layout/AppHeader", () => ({
  AppHeader: () => <header data-testid="app-header" />,
}));

vi.mock("@/components/layout/AppFooter", () => ({
  AppFooter: () => <footer data-testid="app-footer" />,
}));

import RootLayout from "@/app/layout";

describe("RootLayout focus styles", () => {
  it("renders a skip-to-main-content link targeting #main-content", () => {
    render(
      <RootLayout>
        <button type="button">Action</button>
      </RootLayout>,
    );

    const skipLink = screen.getByRole("link", { name: /skip to main content/i });
    expect(skipLink).toHaveAttribute("href", "#main-content");
    expect(skipLink.className).toContain("focus:bg-accent-primary");
    expect(skipLink.className).toContain("focus:text-bg-primary");
    expect(skipLink.className).toContain("focus-visible:outline-accent-secondary");

    const main = screen.getByRole("main");
    expect(main).toHaveAttribute("id", "main-content");
    expect(main).toHaveAttribute("tabindex", "-1");
  });

  it("applies consistent theme-token focus-visible classes on body", () => {
    render(
      <RootLayout>
        <div>Content</div>
      </RootLayout>,
    );

    expect(document.body.className).toContain("[&_a:focus-visible]:outline-accent-primary");
    expect(document.body.className).toContain("[&_button:focus-visible]:outline-accent-primary");
    expect(document.body.className).toContain("[&_select:focus-visible]:outline-accent-primary");
    expect(document.body.className).toContain("[&_textarea:focus-visible]:outline-accent-primary");
    expect(document.body.className).toContain(
      "[&_input:not(.sr-only):focus-visible]:outline-accent-primary",
    );
  });

  it("defines global :focus-visible outline rules using theme tokens in globals.css", () => {
    const css = fs.readFileSync(path.resolve(process.cwd(), "app/globals.css"), "utf8");
    expect(css).toContain(":focus-visible");
    expect(css).toContain("outline: 2px solid var(--accent-primary)");
    expect(css).toContain("outline-offset: 2px");
  });
});
