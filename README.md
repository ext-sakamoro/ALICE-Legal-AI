# ALICE Legal AI

AI-powered legal document analysis, template compilation, and risk scoring.

**License: AGPL-3.0**

---

## Architecture

```
                    ┌─────────────────┐
                    │   Browser / UI  │
                    │  Next.js :3000  │
                    └────────┬────────┘
                             │ HTTP
                    ┌────────▼────────┐
                    │   API Gateway   │
                    │     :8080       │
                    └────────┬────────┘
                             │ HTTP
                    ┌────────▼────────┐
                    │  Legal Engine   │
                    │  Rust/Axum      │
                    │    :8081        │
                    └─────────────────┘
```

| Service | Port | Description |
|---------|------|-------------|
| Frontend | 3000 | Next.js dashboard |
| API Gateway | 8080 | Reverse proxy / auth |
| Legal Engine | 8081 | Rust/Axum core engine |

---

## API Endpoints

### POST /api/v1/legal/analyze

Analyze a legal document for clauses and issues.

**Request:**
```json
{
  "document": "This Agreement is entered into between...",
  "language": "en"
}
```

**Response:**
```json
{
  "risk_score": 0.62,
  "clauses": [
    {
      "id": "clause-001",
      "text": "...",
      "clause_type": "Jurisdiction",
      "risk_level": "low"
    }
  ],
  "issues": [
    {
      "id": "issue-001",
      "description": "Ambiguous indemnification clause detected.",
      "severity": "high",
      "location": "Section 4.2"
    }
  ],
  "language": "en",
  "word_count": 1240
}
```

---

### POST /api/v1/legal/compile

Compile a legal template with variable substitution.

**Request:**
```json
{
  "template_id": "nda",
  "variables": {
    "party_a": "Acme Corp",
    "party_b": "Beta Inc",
    "effective_date": "2026-03-01",
    "jurisdiction": "California"
  }
}
```

**Response:**
```json
{
  "template_id": "nda",
  "compiled_document": "NON-DISCLOSURE AGREEMENT\n\nThis Agreement...",
  "variables_applied": 4,
  "missing_variables": []
}
```

---

### GET /api/v1/legal/templates

List all available legal templates.

**Response:**
```json
{
  "templates": [
    {
      "id": "nda",
      "name": "Non-Disclosure Agreement",
      "description": "...",
      "required_variables": ["party_a", "party_b", "effective_date", "jurisdiction"],
      "language_support": ["en", "ja", "de"]
    }
  ],
  "count": 7
}
```

Available templates: `nda`, `sla`, `dpa`, `tos`, `privacy`, `employment`, `license`

---

### POST /api/v1/legal/risk-score

Compute a detailed risk score breakdown.

**Request:**
```json
{
  "document": "This Service Agreement..."
}
```

**Response:**
```json
{
  "overall_score": 0.58,
  "risk_level": "high",
  "risk_factors": [
    {
      "factor": "Liability Clauses",
      "weight": 0.30,
      "score": 0.80,
      "description": "..."
    }
  ],
  "recommendations": [
    "Engage qualified legal counsel before signing."
  ]
}
```

Risk levels: `low` (< 0.3) | `medium` (0.3–0.5) | `high` (0.5–0.7) | `critical` (>= 0.7)

---

### GET /health

```json
{
  "status": "ok",
  "uptime_secs": 3600,
  "service": "alice-legal-engine",
  "version": "1.0.0"
}
```

---

## Quick Start

### Legal Engine (Rust)

```bash
cd services/core-engine
cargo build --release
LEGAL_ADDR=0.0.0.0:8081 ./target/release/legal-engine
```

### Frontend (Next.js)

```bash
cd frontend
npm install
NEXT_PUBLIC_LEGAL_API_URL=http://localhost:8081 npm run dev
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LEGAL_ADDR` | `0.0.0.0:8081` | Legal engine bind address |
| `NEXT_PUBLIC_LEGAL_API_URL` | `http://localhost:8081` | API base URL for frontend |

---

## Supported Languages

| Code | Language |
|------|----------|
| `en` | English |
| `ja` | Japanese |
| `de` | German |
| `fr` | French |

---

## Disclaimer

ALICE Legal AI is a software tool and does not constitute legal advice. Always consult a qualified attorney before signing or enforcing any legal document.

---

## License

AGPL-3.0 — See [LICENSE](LICENSE) for details.
