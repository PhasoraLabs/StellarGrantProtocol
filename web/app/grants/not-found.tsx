import Link from "next/link";

export default function GrantNotFound() {
  return (
    <div className="container mx-auto max-w-2xl px-4 py-24 text-center">
      <p className="font-orbitron text-4xl font-black text-accent-primary mb-4">
        Grant Not Found
      </p>
      <p className="font-mono text-sm text-text-muted mb-8 leading-relaxed">
        Grant not found — it may have been cancelled or the ID is incorrect.
      </p>
      <Link
        href="/grants"
        className="inline-flex items-center font-orbitron text-sm font-bold uppercase tracking-wider border border-accent-primary text-accent-primary px-8 py-3 hover:bg-accent-primary hover:text-bg-primary transition-colors"
      >
        Browse all grants →
      </Link>
    </div>
  );
}
