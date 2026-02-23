"use client";

import { useState } from "react";
import { useLegalStore } from "@/lib/hooks/use-store";
import { legalClient } from "@/lib/api/client";

const LANGUAGES = [
  { value: "en", label: "English" },
  { value: "ja", label: "Japanese" },
  { value: "de", label: "German" },
  { value: "fr", label: "French" },
];

const TEMPLATES = [
  { value: "nda", label: "Non-Disclosure Agreement (NDA)" },
  { value: "sla", label: "Service Level Agreement (SLA)" },
  { value: "dpa", label: "Data Processing Agreement (DPA)" },
  { value: "tos", label: "Terms of Service (ToS)" },
  { value: "privacy", label: "Privacy Policy" },
  { value: "employment", label: "Employment Agreement" },
  { value: "license", label: "Software License Agreement" },
];

function RiskBadge({ score }: { score: number }) {
  const level =
    score >= 0.7 ? "critical" : score >= 0.5 ? "high" : score >= 0.3 ? "medium" : "low";
  const colors: Record<string, string> = {
    critical: "bg-red-500/20 text-red-400 border-red-500/40",
    high: "bg-orange-500/20 text-orange-400 border-orange-500/40",
    medium: "bg-yellow-500/20 text-yellow-400 border-yellow-500/40",
    low: "bg-green-500/20 text-green-400 border-green-500/40",
  };
  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs border ${colors[level]}`}
    >
      {level.toUpperCase()} {(score * 100).toFixed(0)}%
    </span>
  );
}

export default function LegalConsolePage() {
  const {
    document,
    setDocument,
    language,
    setLanguage,
    templateId,
    setTemplateId,
    result,
    setResult,
    loading,
    setLoading,
  } = useLegalStore();

  const [activeTab, setActiveTab] = useState<"analyze" | "compile">("analyze");
  const [error, setError] = useState<string | null>(null);

  async function handleAnalyze() {
    if (!document.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const res = await legalClient.analyze({ document, language });
      setResult({ type: "analyze", data: res });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Analysis failed.");
    } finally {
      setLoading(false);
    }
  }

  async function handleCompile() {
    if (!templateId) return;
    setLoading(true);
    setError(null);
    try {
      const res = await legalClient.compile({
        template_id: templateId,
        variables: {},
      });
      setResult({ type: "compile", data: res });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Compilation failed.");
    } finally {
      setLoading(false);
    }
  }

  async function handleRiskScore() {
    if (!document.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const res = await legalClient.riskScore({ document });
      setResult({ type: "risk", data: res });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Risk scoring failed.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen bg-[#0a0a0f] text-white">
      {/* Header */}
      <header className="border-b border-white/10 px-6 py-4 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-violet-400">Legal Console</h1>
          <p className="text-xs text-gray-500 mt-0.5">ALICE Legal AI — Document Analysis</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => setActiveTab("analyze")}
            className={`px-3 py-1.5 rounded-md text-sm transition-colors ${
              activeTab === "analyze"
                ? "bg-violet-600 text-white"
                : "text-gray-400 hover:text-white"
            }`}
          >
            Analyze
          </button>
          <button
            onClick={() => setActiveTab("compile")}
            className={`px-3 py-1.5 rounded-md text-sm transition-colors ${
              activeTab === "compile"
                ? "bg-violet-600 text-white"
                : "text-gray-400 hover:text-white"
            }`}
          >
            Compile
          </button>
        </div>
      </header>

      <div className="max-w-6xl mx-auto px-6 py-8 grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Controls */}
        <div className="space-y-5">
          {activeTab === "analyze" && (
            <>
              {/* Language selector */}
              <div>
                <label className="block text-xs text-gray-400 mb-1.5 uppercase tracking-widest">
                  Document Language
                </label>
                <select
                  value={language}
                  onChange={(e) => setLanguage(e.target.value)}
                  className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-violet-500"
                >
                  {LANGUAGES.map((l) => (
                    <option key={l.value} value={l.value} className="bg-[#1a1a2e]">
                      {l.label}
                    </option>
                  ))}
                </select>
              </div>

              {/* Document textarea */}
              <div>
                <label className="block text-xs text-gray-400 mb-1.5 uppercase tracking-widest">
                  Contract / Document
                </label>
                <textarea
                  value={document}
                  onChange={(e) => setDocument(e.target.value)}
                  placeholder="Paste your contract, NDA, SLA, or any legal document here..."
                  rows={16}
                  className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white placeholder-gray-600 focus:outline-none focus:border-violet-500 resize-none font-mono leading-relaxed"
                />
              </div>

              {/* Actions */}
              <div className="flex gap-3">
                <button
                  onClick={handleAnalyze}
                  disabled={loading || !document.trim()}
                  className="flex-1 py-2.5 rounded-lg bg-violet-600 hover:bg-violet-500 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-semibold transition-colors"
                >
                  {loading ? "Analyzing..." : "Analyze Document"}
                </button>
                <button
                  onClick={handleRiskScore}
                  disabled={loading || !document.trim()}
                  className="flex-1 py-2.5 rounded-lg border border-white/20 hover:border-violet-500/60 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-semibold transition-colors"
                >
                  Risk Score
                </button>
              </div>
            </>
          )}

          {activeTab === "compile" && (
            <>
              {/* Template selector */}
              <div>
                <label className="block text-xs text-gray-400 mb-1.5 uppercase tracking-widest">
                  Template
                </label>
                <select
                  value={templateId}
                  onChange={(e) => setTemplateId(e.target.value)}
                  className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-violet-500"
                >
                  <option value="" className="bg-[#1a1a2e]">
                    Select a template...
                  </option>
                  {TEMPLATES.map((t) => (
                    <option key={t.value} value={t.value} className="bg-[#1a1a2e]">
                      {t.label}
                    </option>
                  ))}
                </select>
              </div>

              <div className="rounded-lg border border-white/10 bg-white/5 p-4">
                <p className="text-xs text-gray-400 leading-relaxed">
                  Select a template to compile. Variables can be provided via the API
                  or the advanced editor. The compiled document will appear in the
                  result panel on the right.
                </p>
              </div>

              <button
                onClick={handleCompile}
                disabled={loading || !templateId}
                className="w-full py-2.5 rounded-lg bg-violet-600 hover:bg-violet-500 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-semibold transition-colors"
              >
                {loading ? "Compiling..." : "Compile Template"}
              </button>
            </>
          )}

          {error && (
            <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-400">
              {error}
            </div>
          )}
        </div>

        {/* Result panel */}
        <div className="rounded-xl border border-white/10 bg-white/5 p-5">
          <h2 className="text-sm font-semibold text-gray-300 mb-4 uppercase tracking-widest">
            Result
          </h2>

          {!result && !loading && (
            <p className="text-sm text-gray-600 italic">
              Run an analysis or compile a template to see results here.
            </p>
          )}

          {loading && (
            <div className="flex items-center gap-3 text-violet-400 text-sm">
              <span className="animate-spin">⟳</span>
              Processing...
            </div>
          )}

          {result && !loading && (
            <div className="space-y-4 text-sm">
              {/* Analyze result */}
              {result.type === "analyze" && (
                <>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">Risk Score</span>
                    <RiskBadge score={result.data.risk_score} />
                  </div>
                  <div className="flex items-center justify-between text-gray-400">
                    <span>Word Count</span>
                    <span className="text-white">{result.data.word_count.toLocaleString()}</span>
                  </div>

                  {result.data.clauses?.length > 0 && (
                    <div>
                      <p className="text-xs text-gray-500 uppercase tracking-widest mb-2">
                        Clauses ({result.data.clauses.length})
                      </p>
                      <div className="space-y-2">
                        {result.data.clauses.map((c: {id: string; clause_type: string; risk_level: string; text: string}) => (
                          <div
                            key={c.id}
                            className="rounded-lg bg-white/5 border border-white/10 p-3"
                          >
                            <div className="flex items-center gap-2 mb-1">
                              <span className="text-xs font-medium text-violet-300">
                                {c.clause_type}
                              </span>
                              <span
                                className={`text-xs px-1.5 py-0.5 rounded ${
                                  c.risk_level === "high"
                                    ? "bg-orange-500/20 text-orange-400"
                                    : c.risk_level === "medium"
                                    ? "bg-yellow-500/20 text-yellow-400"
                                    : "bg-green-500/20 text-green-400"
                                }`}
                              >
                                {c.risk_level}
                              </span>
                            </div>
                            <p className="text-xs text-gray-400 line-clamp-2">{c.text}</p>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {result.data.issues?.length > 0 && (
                    <div>
                      <p className="text-xs text-gray-500 uppercase tracking-widest mb-2">
                        Issues ({result.data.issues.length})
                      </p>
                      <div className="space-y-2">
                        {result.data.issues.map((i: {id: string; description: string; severity: string; location: string}) => (
                          <div
                            key={i.id}
                            className="rounded-lg bg-red-500/5 border border-red-500/20 p-3"
                          >
                            <div className="flex items-center justify-between mb-1">
                              <span className="text-xs text-red-400">{i.severity}</span>
                              <span className="text-xs text-gray-500">{i.location}</span>
                            </div>
                            <p className="text-xs text-gray-300">{i.description}</p>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}

              {/* Risk result */}
              {result.type === "risk" && (
                <>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-400">Overall Risk</span>
                    <RiskBadge score={result.data.overall_score} />
                  </div>

                  {result.data.risk_factors?.map((f: {factor: string; score: number; description: string}) => (
                    <div key={f.factor}>
                      <div className="flex items-center justify-between text-xs mb-1">
                        <span className="text-gray-400">{f.factor}</span>
                        <span className="text-white">{(f.score * 100).toFixed(0)}%</span>
                      </div>
                      <div className="h-1.5 rounded-full bg-white/10 overflow-hidden">
                        <div
                          className="h-full bg-violet-500 rounded-full"
                          style={{ width: `${f.score * 100}%` }}
                        />
                      </div>
                    </div>
                  ))}

                  {result.data.recommendations?.length > 0 && (
                    <div className="pt-2">
                      <p className="text-xs text-gray-500 uppercase tracking-widest mb-2">
                        Recommendations
                      </p>
                      <ul className="space-y-1">
                        {result.data.recommendations.map((r: string, i: number) => (
                          <li key={i} className="text-xs text-gray-300 flex gap-2">
                            <span className="text-violet-400 shrink-0">→</span>
                            {r}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </>
              )}

              {/* Compile result */}
              {result.type === "compile" && (
                <>
                  <div className="flex items-center justify-between text-gray-400">
                    <span>Template</span>
                    <span className="text-white">{result.data.template_id}</span>
                  </div>
                  <div className="flex items-center justify-between text-gray-400">
                    <span>Variables Applied</span>
                    <span className="text-green-400">{result.data.variables_applied}</span>
                  </div>
                  {result.data.missing_variables?.length > 0 && (
                    <div className="flex items-center justify-between text-gray-400">
                      <span>Missing Variables</span>
                      <span className="text-yellow-400">
                        {result.data.missing_variables.join(", ")}
                      </span>
                    </div>
                  )}
                  <div>
                    <p className="text-xs text-gray-500 uppercase tracking-widest mb-2">
                      Compiled Document
                    </p>
                    <pre className="text-xs text-gray-300 bg-black/30 rounded-lg p-3 whitespace-pre-wrap leading-relaxed max-h-64 overflow-y-auto">
                      {result.data.compiled_document}
                    </pre>
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
