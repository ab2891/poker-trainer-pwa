/**
 * AboutFooter.tsx — About + product info.
 */

export function AboutFooter() {
  return (
    <div className="w-full max-w-2xl flex flex-col items-center gap-12 mt-8">
      <header className="text-center">
        <p className="text-fg-subtle font-mono text-xs uppercase tracking-widest mb-3">
          About
        </p>
        <h1 className="font-serif text-4xl sm:text-5xl text-fg leading-tight">
          Free poker training.
          <br />
          <span className="italic">No paywalls. Ever.</span>
        </h1>
        <p className="text-fg-muted mt-6 max-w-md mx-auto text-sm leading-relaxed">
          Poker Trainer is an interactive preflop training app for no-limit
          hold'em. You get randomized spots, immediate EV-based feedback, and
          explanations designed to help you learn position, ranges, and
          decision quality over time.
        </p>
      </header>

      <section className="glass px-7 py-5 max-w-xl w-full">
        <p className="text-fg-subtle font-mono text-[10px] uppercase tracking-widest mb-2">
          What You Get
        </p>
        <p className="text-fg-muted text-sm leading-relaxed">
          Practice first-in opens, defense versus opens, and defense versus
          3-bets and squeezes. The trainer uses a Rust engine and returns the
          highest-EV action for each generated preflop node under the current
          stack and rake settings.
        </p>
      </section>

      <div className="glass glass-strong px-8 py-6 flex flex-col items-center gap-3 max-w-sm w-full">
        <p className="text-fg-subtle font-mono text-[10px] uppercase tracking-widest">
          Product Status
        </p>
        <p className="text-fg text-center text-sm leading-relaxed">
          Live preflop training product with ongoing feature updates.
        </p>
        <a
          href="https://ko-fi.com/ab2891"
          target="_blank"
          rel="noreferrer"
          className="text-fg font-serif text-2xl hover:text-accent transition-colors duration-200"
        >
          ko-fi.com/ab2891
        </a>
        <p className="text-fg-faint text-xs text-center">
          Practice is available directly in the app. Commercial offerings and
          advanced tiers are published on official release channels.
        </p>
      </div>

      <footer className="text-center pb-12">
        <p className="text-fg-faint font-mono text-xs">
          Made by{" "}
          <a
            href="https://github.com/ab2891"
            target="_blank"
            rel="noreferrer"
            className="hover:text-fg-muted transition-colors"
          >
            ab2891
          </a>
          {" · "}
          Engine in Rust →{" "}
          <a
            href="https://github.com/ab2891/pokerTrainer"
            target="_blank"
            rel="noreferrer"
            className="hover:text-fg-muted transition-colors"
          >
            source
          </a>
        </p>
      </footer>
    </div>
  );
}
