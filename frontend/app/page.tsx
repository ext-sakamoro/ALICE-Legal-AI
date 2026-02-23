"use client";

import Link from "next/link";

const features = [
  {
    title: "Contract Analysis",
    description:
      "Upload any contract and receive a detailed clause-by-clause breakdown with risk annotations. Supports NDA, SLA, DPA, ToS, and more.",
    icon: "📄",
  },
  {
    title: "Template Compilation",
    description:
      "Fill in variables to compile production-ready legal documents from ALICE-vetted templates in seconds.",
    icon: "🔧",
  },
  {
    title: "Risk Scoring",
    description:
      "Quantitative risk score across liability, indemnification, IP assignment, and termination clauses with actionable recommendations.",
    icon: "⚖️",
  },
];

export default function HomePage() {
  return (
    <main className="min-h-screen bg-[#0a0a0f] text-white font-sans">
      {/* Nav */}
      <nav className="border-b border-white/10 px-6 py-4 flex items-center justify-between">
        <span className="text-lg font-bold tracking-tight text-violet-400">
          ALICE Legal AI
        </span>
        <Link
          href="/dashboard/console"
          className="text-sm px-4 py-2 rounded-md bg-violet-600 hover:bg-violet-500 transition-colors"
        >
          Open Console
        </Link>
      </nav>

      {/* Hero */}
      <section className="max-w-4xl mx-auto px-6 pt-28 pb-16 text-center">
        <div className="inline-block mb-4 px-3 py-1 rounded-full border border-violet-500/40 bg-violet-500/10 text-violet-300 text-xs tracking-widest uppercase">
          Powered by ALICE-Legal
        </div>
        <h1 className="text-5xl md:text-6xl font-extrabold tracking-tight mb-6 leading-tight">
          ALICE Legal AI
        </h1>
        <p className="text-xl md:text-2xl text-violet-300 font-medium mb-4">
          Don&apos;t read contracts blindly.
          <br />
          <span className="text-white">Read the law of law.</span>
        </p>
        <p className="text-gray-400 max-w-xl mx-auto mb-10">
          AI-powered legal analysis powered by ALICE-Legal. Analyze contracts,
          compile templates, and surface risk factors in multiple languages
          — without a law degree.
        </p>
        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <Link
            href="/dashboard/console"
            className="px-8 py-3 rounded-lg bg-violet-600 hover:bg-violet-500 font-semibold transition-colors"
          >
            Launch Console
          </Link>
          <a
            href="#features"
            className="px-8 py-3 rounded-lg border border-white/20 hover:border-white/40 font-semibold transition-colors"
          >
            Learn More
          </a>
        </div>
      </section>

      {/* Features */}
      <section id="features" className="max-w-5xl mx-auto px-6 py-20">
        <h2 className="text-center text-3xl font-bold mb-12 text-white">
          What ALICE Legal AI Does
        </h2>
        <div className="grid md:grid-cols-3 gap-6">
          {features.map((f) => (
            <div
              key={f.title}
              className="rounded-xl border border-white/10 bg-white/5 p-6 hover:border-violet-500/50 transition-colors"
            >
              <div className="text-3xl mb-4">{f.icon}</div>
              <h3 className="text-lg font-semibold mb-2 text-white">
                {f.title}
              </h3>
              <p className="text-sm text-gray-400 leading-relaxed">
                {f.description}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Templates strip */}
      <section className="border-t border-white/10 py-12">
        <div className="max-w-4xl mx-auto px-6 text-center">
          <p className="text-sm text-gray-500 uppercase tracking-widest mb-4">
            Supported Templates
          </p>
          <div className="flex flex-wrap justify-center gap-3">
            {["NDA", "SLA", "DPA", "Terms of Service", "Privacy Policy", "Employment", "License"].map(
              (t) => (
                <span
                  key={t}
                  className="px-3 py-1 rounded-full text-xs border border-white/20 text-gray-300"
                >
                  {t}
                </span>
              )
            )}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="max-w-2xl mx-auto px-6 py-20 text-center">
        <h2 className="text-3xl font-bold mb-4">
          Start analyzing your contracts today.
        </h2>
        <p className="text-gray-400 mb-8">
          No law degree required. ALICE does the reading.
        </p>
        <Link
          href="/dashboard/console"
          className="inline-block px-10 py-3 rounded-lg bg-violet-600 hover:bg-violet-500 font-semibold transition-colors"
        >
          Open Legal Console
        </Link>
      </section>

      {/* Footer */}
      <footer className="border-t border-white/10 px-6 py-8 text-center text-xs text-gray-600">
        ALICE Legal AI — AGPL-3.0 — Not a substitute for qualified legal advice.
      </footer>
    </main>
  );
}
