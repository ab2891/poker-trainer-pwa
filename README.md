# poker-trainer-pwa

A free, no-paywall, no-ads preflop GTO trainer that ships as a PWA. The Rust GTO engine from [`pokerTrainer`](https://github.com/ab2891/poker-trainer) compiles to WebAssembly and runs locally in the browser. Designed to be installable on iPhone via "Add to Home Screen" — no App Store, no $99/year, no Apple gatekeeping.

## Repo layout

```
poker-trainer-pwa/
├── engine/              Rust crate. Copies of model.rs + charts.rs from
│                        pokerTrainer with serde derives added on the public
│                        types and a wasm-bindgen boundary in src/wasm.rs.
│                        Builds to WASM via wasm-pack.
│   ├── Cargo.toml
│   ├── data/
│   │   └── preflop_charts.json   GTO chart data, copied from pokerTrainer
│   └── src/
│       ├── lib.rs       Re-exports
│       ├── model.rs     Engine logic (Position, Card, TrainingSpot, etc)
│       ├── charts.rs    Chart book lookup
│       └── wasm.rs      JS-facing API (init, default_config, generate_spot,
│                        evaluate_action, training_modes)
└── web/                 React + Vite + TS + Tailwind 4 PWA.
    ├── package.json
    ├── vite.config.ts   Plugins: react, wasm, tailwindcss, PWA service worker
    ├── tsconfig.json    Strict mode
    ├── index.html       PWA meta tags
    └── src/
        ├── main.tsx
        ├── App.tsx
        ├── index.css    Design tokens — Fluxly-style charcoal/iridium glass
        ├── engine.ts    Typed wrapper around the WASM API
        ├── components/
        │   ├── Header.tsx        Placeholder glass nav pill
        │   ├── DrillScreen.tsx   Placeholder end-to-end drill flow
        │   └── AboutFooter.tsx   Placeholder About + Ko-fi link
        └── wasm/        wasm-pack output (gitignored from source; rebuild
                         with `npm run engine:build`)
```

## Prerequisites

- Rust + Cargo (you have this)
- `wasm-pack` (`cargo install wasm-pack` if missing)
- Node 20+ and npm

## First-time setup

```bash
cd web
npm run engine:build      # compiles engine/ to web/src/wasm/
npm install
npm run dev               # http://localhost:5173/
```

Or, in two terminals:

```bash
# terminal 1 — rebuild WASM whenever you touch the engine
cd engine
cargo watch -x 'check' -s '../web/node_modules/.bin/vite-plugin-wasm-rebuild' 2>/dev/null || \
  cd ../web && npm run engine:build

# terminal 2 — vite dev server
cd web && npm run dev
```

## How the WASM ↔ React boundary works

The Rust engine is exposed to JavaScript through `engine/src/wasm.rs`, which uses [`serde-wasm-bindgen`](https://github.com/cloudflare/serde-wasm-bindgen) to flatten Rust structs into idiomatic JS objects. The TypeScript wrapper in `web/src/engine.ts` re-types those objects with `interface` declarations that mirror the Rust types in `engine/src/model.rs`.

The React side calls four typed functions, all returning Promises:

```ts
import {
  defaultConfig, generateSpot, evaluateAction, trainingModes,
} from "./engine";

const cfg = await defaultConfig();
const spot = await generateSpot(cfg);
const feedback = await evaluateAction(spot, "Raise");
const modes = await trainingModes();
```

The `await`s exist because the WASM module needs to be loaded the first time. After that they resolve synchronously fast.

## What you need to do

The scaffold ships with a deliberately ugly placeholder UI in `web/src/components/`. Every visual decision (typography, layout, motion, card faces, spacing, color of buttons) is explicitly marked `TODO: replace this`. The point of the scaffold is to:

1. Prove the engine wiring works end-to-end
2. Ship the design tokens (charcoal/iridium glass palette in `index.css`) you can build against
3. Give you a working `npm run dev` from minute one

You should:
- Redesign `Header.tsx`, `DrillScreen.tsx`, `AboutFooter.tsx` from scratch
- Replace the placeholder `<CardFace>` with real card art
- Replace the bland `<ActionButton>` with whatever you actually want
- Add session stats / settings / mode selector / chart browser as you need them

The engine layer in `engine.ts` and the design tokens in `index.css` are stable foundations — touch them only if you want to add new tokens or new engine functions.

## Updating the engine

If you edit anything in `engine/src/`, rebuild the WASM:

```bash
cd web && npm run engine:build
```

Vite will hot-reload the result automatically.

If you make breaking changes to the engine API, also update the matching types in `web/src/engine.ts`.

## Distributing

Once you're happy:

```bash
cd web && npm run build
```

The output in `web/dist/` is a static site you can host anywhere. Recommended: Cloudflare Pages (free, fast, PWA-friendly). Drop the `dist/` folder into a Pages project, point a domain at it, done.

For App Store submission later (if you ever get Mac access): wrap `web/dist/` with [Capacitor](https://capacitorjs.com), open in Xcode, submit. ~2 hours of work.

## Tip jar (Ko-fi)

The placeholder About page links to `https://ko-fi.com/ab2891`. Replace that with your real Ko-fi URL when you create the page. Ko-fi handles all payment processing — you never see card details. PCI compliance is on Ko-fi.

## License

MIT.
