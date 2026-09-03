/**
 * Sidebar Component
 * 
 * Collapsible left sidebar for grant detail pages with anchor links
 * to page sections.
 */

interface SidebarProps {
  sections?: Array<{ id: string; label: string }>;
}

export function Sidebar({ sections = [] }: SidebarProps) {
  return (
    <aside className="w-64 border-r p-4">
      <nav className="space-y-2">
        {sections.map((section) => (
          <a
            key={section.id}
            href={`#${section.id}`}
            className="block text-sm text-muted-foreground hover:text-foreground"
          >
            {section.label}
          </a>
        ))}
      </nav>
    </aside>
  );
}
