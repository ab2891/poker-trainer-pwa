/**
 * AboutFooter.tsx — placeholder About + tip-jar screen.
 *
 * REPLACE THIS WITH YOUR OWN COPY. The structure here is just a frame:
 *  - A hero "what is this" paragraph
 *  - A tip-jar block linking to Ko-fi (placeholder URL!)
 *  - A small "made by" line
 *
 * IMPORTANT: replace `https://ko-fi.com/ab2891` with your real Ko-fi URL once
 * you create the account. Same for any GitHub Sponsors / Patreon links.
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
          {/* TODO: rewrite this in your voice. */}
          A preflop GTO trainer that doesn't lock features behind a subscription
          and doesn't run ads. Drill, learn, get better. If it's useful and you
          want to throw a few dollars in the tip jar, the link's below.
        </p>
      </header>

      <div className="glass glass-strong px-8 py-6 flex flex-col items-center gap-3 max-w-sm w-full">
        <p className="text-fg-subtle font-mono text-[10px] uppercase tracking-widest">
          Tip jar — entirely optional
        </p>
        <a
          // TODO: replace with your real Ko-fi URL once you create the page.
          href="https://ko-fi.com/ab2891"
          target="_blank"
          rel="noreferrer"
          className="text-fg font-serif text-2xl hover:text-accent transition-colors duration-200"
        >
          ko-fi.com/ab2891
        </a>
        <p className="text-fg-faint text-xs text-center">
          Card details never touch this site. Ko-fi handles everything.
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
