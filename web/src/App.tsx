/**
 * App.tsx — root React component.
 *
 * This is intentionally a thin shell so you (Agustin) can replace any of these
 * pieces with your own design. The pieces in this scaffold:
 *
 *  - <Header /> — placeholder glass nav pill at the top
 *  - <DrillScreen /> — placeholder showing one drill end-to-end so you can
 *    confirm the engine wiring works. REPLACE THIS WITH YOUR OWN UI.
 *  - <AboutFooter /> — product/about information
 *
 * Your job: redesign each of these (or replace them entirely) with the
 * Fluxly-style aesthetic you described. The engine layer in `engine.ts`
 * gives you everything you need (`generateSpot`, `evaluateAction`,
 * `trainingModes`, etc.) — just import and call.
 */

import { useState } from "react";
import { DrillScreen } from "./components/DrillScreen";
import { Header } from "./components/Header";
import { AboutFooter } from "./components/AboutFooter";

export default function App() {
  const [view, setView] = useState<"drill" | "about">("drill");

  return (
    <div className="glass-stage min-h-screen flex flex-col">
      <Header currentView={view} onNavigate={setView} />
      <main className="relative z-10 flex-1 flex flex-col items-center justify-start px-4 py-8 sm:py-16">
        {view === "drill" && <DrillScreen />}
        {view === "about" && <AboutFooter />}
      </main>
    </div>
  );
}
