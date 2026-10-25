# ALICE Legal AI

AI-powered legal document analysis, template compilation, and risk scoring.

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
  "word_count": 1240,
  "audit_head": "9c41…"
}
```

Each analysis is one entry in the audit log; `audit_head` is the log head after it. A refused document is recorded too (see the audit log below).

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
  },
  "counterparty_did": "did:key:z6Mk…"
}
```

`counterparty_did` is required and must be a DID (`did:<method>:<id>`); otherwise the request is refused with 400.

**Response:**
```json
{
  "template_id": "nda",
  "compiled_document": "NON-DISCLOSURE AGREEMENT\n\nThis Agreement...",
  "variables_applied": 4,
  "missing_variables": [],
  "signed": {
    "id": "1-1",
    "kind": "template-compiled",
    "issuer_did": "did:key:z6Mk…",
    "counterparty_did": "did:key:z6Mk…",
    "effective_unix": 1792886400,
    "metadata": { "document_sha256": "5f0e…", "template_id": "nda" },
    "digest": "a3b2…",
    "public_key": "7d4c…",
    "signature": "e1f0…"
  },
  "audit_head": "2b77…"
}
```

When no variable is missing, the engine signs the record with its Ed25519 key (ALICE-Legal `SignedContract`). The record's metadata holds the SHA-256 of `compiled_document`; `signature` is over the record's canonical bytes and verifies under `public_key`; `issuer_did` is the `did:key` of that key (ALICE-Identity `Did::key_ed25519`). `id` is `<log_id>-<sequence>`, the sequence of the audit entry that records this compile, assigned by that entry's append (ALICE-Legal `AuditLog::append_with`); the record is signed while the log is held, so concurrent requests cannot take that sequence. When a variable is missing, `signed` is `null`. Either way the request is one entry in the audit log, and `audit_head` is the head after it.

---

### GET /api/v1/legal/audit

The audit log (ALICE-Legal `AuditLog`): each entry is linked to the one before it, and `head` is the link the next entry would carry.

**Response:**
```json
{
  "log_id": 1,
  "head": "2b77…",
  "entries": [
    {
      "sequence": 0,
      "kind": "document-analyzed",
      "entity_id": 0,
      "actor_hash": "41d0…",
      "timestamp_ns": 1792886400000000000,
      "content": "{\"document_sha256\":\"5f0e…\",\"outcome\":\"analyzed\",\"request\":\"analyze\"}",
      "content_hash": "c8a1…",
      "prev_hash": "6e02…"
    }
  ]
}
```

Every analyze, risk-score and compile request the engine answers is one entry, including a refused one. `content` is the request and its outcome as JSON with sorted keys, and `content_hash` is its identifier (ALICE-Legal `law_id::text("alice-legal/audit-entry/content/v1", content)`):

| request | kind | `outcome` | other fields |
|---------|------|-----------|--------------|
| analyze | `document-analyzed` | `analyzed` | `document_sha256` |
| risk-score | `document-analyzed` | `scored` | `document_sha256` |
| analyze, risk-score | `document-analyzed` | `refused` (`reason`: `no-clauses`) | `document_sha256` |
| compile | `template-compiled` | `signed` | `template_id`, `digest` (of the signed record) |
| compile | `template-compiled` | `incomplete` | `template_id`, `missing` |
| compile | `template-compiled` | `refused` (`reason`: `empty-template-id`, `counterparty-not-a-did` or `unknown-template`) | `template_id` |
| analyze, risk-score | `document-analyzed` | `refused` (`reason`: `malformed-body`) | none |
| compile | `template-compiled` | `refused` (`reason`: `malformed-body`) | none |

`malformed-body` is a body the JSON extractor refuses (not JSON, a missing field, another content type); it is answered with the extractor's status. Every entry's `entity_id` is 0 (the engine has no numeric entity). A refused request is answered with its status code only; its entry and the head after it are in this log. `/health`, `/api/v1/legal/templates` and this endpoint are not recorded: they read no document and change nothing.

The log is checked against a head kept outside it: keep the `audit_head` values the responses return. Against the log's own head, a truncated log still verifies. The log is held in memory and starts empty at each start of the engine.

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
  ],
  "audit_head": "7a19…"
}
```

Each scored document is one entry in the audit log; `audit_head` is the log head after it.

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
LEGAL_SIGNING_SEED=<64 hex characters> LEGAL_AUDIT_LOG_ID=1 LEGAL_ADDR=0.0.0.0:8081 ./target/release/legal-engine
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
| `LEGAL_SIGNING_SEED` | none (required) | The engine's Ed25519 signing key: the 32-byte RFC 8032 seed as 64 hex characters |
| `LEGAL_AUDIT_LOG_ID` | none (required) | The audit log id (unsigned 64-bit integer) |
| `NEXT_PUBLIC_LEGAL_API_URL` | `http://localhost:8081` | API base URL for frontend |

### Signing key and audit log

The engine does not start without `LEGAL_SIGNING_SEED` and `LEGAL_AUDIT_LOG_ID`: it prints which variable is missing or malformed and exits with status 2. The seed text and bytes are wiped once the key is derived, and no message prints any part of the seed. `docker-compose.yml` requires both variables (`${VAR:?}`) and sets no value for them; provide them from your environment or secret store.

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

`LicenseRef-Proprietary`. The text is [LICENSE-Proprietary](LICENSE-Proprietary).

Enquiries: https://extoria.co.jp/
