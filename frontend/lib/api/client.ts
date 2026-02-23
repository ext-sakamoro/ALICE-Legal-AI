const BASE_URL =
  process.env.NEXT_PUBLIC_LEGAL_API_URL ?? "http://localhost:8081";

// ── Request types ─────────────────────────────────────────────────────────────

export interface AnalyzeRequest {
  document: string;
  language: string;
}

export interface CompileRequest {
  template_id: string;
  variables: Record<string, string>;
}

export interface RiskRequest {
  document: string;
}

// ── Response types ────────────────────────────────────────────────────────────

export interface Clause {
  id: string;
  text: string;
  clause_type: string;
  risk_level: string;
}

export interface Issue {
  id: string;
  description: string;
  severity: string;
  location: string;
}

export interface AnalyzeResponse {
  risk_score: number;
  clauses: Clause[];
  issues: Issue[];
  language: string;
  word_count: number;
}

export interface CompileResponse {
  template_id: string;
  compiled_document: string;
  variables_applied: number;
  missing_variables: string[];
}

export interface TemplateInfo {
  id: string;
  name: string;
  description: string;
  required_variables: string[];
  language_support: string[];
}

export interface TemplatesResponse {
  templates: TemplateInfo[];
  count: number;
}

export interface RiskFactor {
  factor: string;
  weight: number;
  score: number;
  description: string;
}

export interface RiskScoreResponse {
  overall_score: number;
  risk_level: string;
  risk_factors: RiskFactor[];
  recommendations: string[];
}

// ── HTTP helper ───────────────────────────────────────────────────────────────

async function request<T>(
  method: "GET" | "POST",
  path: string,
  body?: unknown
): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`${res.status} ${res.statusText}${text ? `: ${text}` : ""}`);
  }

  return res.json() as Promise<T>;
}

// ── LegalClient ───────────────────────────────────────────────────────────────

export const legalClient = {
  /**
   * Analyze a legal document and return risk score, clauses, and issues.
   */
  async analyze(req: AnalyzeRequest): Promise<AnalyzeResponse> {
    return request<AnalyzeResponse>("POST", "/api/v1/legal/analyze", req);
  },

  /**
   * Compile a legal template by substituting provided variables.
   */
  async compile(req: CompileRequest): Promise<CompileResponse> {
    return request<CompileResponse>("POST", "/api/v1/legal/compile", req);
  },

  /**
   * Retrieve the list of all available legal templates.
   */
  async templates(): Promise<TemplatesResponse> {
    return request<TemplatesResponse>("GET", "/api/v1/legal/templates");
  },

  /**
   * Compute a quantitative risk score for the provided document.
   */
  async riskScore(req: RiskRequest): Promise<RiskScoreResponse> {
    return request<RiskScoreResponse>("POST", "/api/v1/legal/risk-score", req);
  },
};
