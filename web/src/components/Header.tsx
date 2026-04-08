/**
 * Header.tsx — placeholder glass nav pill.
 *
 * The Fluxly screenshot has a thin centered pill with brand on the left and
 * nav links on the right. This placeholder mimics the *shape* but uses
 * deliberately bland copy and zero brand opinion so you'll replace it.
 *
 * Things to redesign:
 *  - The logotype on the left (currently the word "trainer" in serif)
 *  - The nav items (currently just Drill / About)
 *  - The CTA position (Fluxly has a paired "Login / Sign up" — you don't
 *    need that since this is a single-user free app, but you might want a
 *    "Settings" or "Stats" button there)
 */

interface HeaderProps {
  currentView: "drill" | "about";
  onNavigate: (view: "drill" | "about") => void;
}

export function Header({ currentView, onNavigate }: HeaderProps) {
  return (
    <header className="relative z-10 px-4 pt-6 sm:pt-8 flex justify-center">
      <nav className="glass flex items-center gap-1 px-2 py-1.5">
        <button
          onClick={() => onNavigate("drill")}
          className="px-4 py-1.5 rounded-full text-sm tracking-tight font-serif italic text-fg cursor-default"
          aria-label="Brand"
        >
          {/* TODO: replace with your logotype */}
          trainer
        </button>
        <div className="w-px h-5 bg-glass-border mx-1" aria-hidden />
        <NavItem
          label="Drill"
          active={currentView === "drill"}
          onClick={() => onNavigate("drill")}
        />
        <NavItem
          label="About"
          active={currentView === "about"}
          onClick={() => onNavigate("about")}
        />
      </nav>
    </header>
  );
}

function NavItem({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={
        "px-4 py-1.5 rounded-full text-sm transition-colors duration-200 " +
        (active
          ? "text-fg bg-bg-glass-strong"
          : "text-fg-muted hover:text-fg")
      }
    >
      {label}
    </button>
  );
}
