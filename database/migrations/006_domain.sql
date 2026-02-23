-- Legal AI domain tables
create table if not exists public.documents (
    id uuid primary key default gen_random_uuid(),
    user_id uuid references auth.users(id) on delete cascade,
    title text not null,
    doc_type text not null check (doc_type in ('contract', 'nda', 'terms', 'privacy_policy', 'license', 'other')),
    content_hash text,
    word_count integer,
    page_count integer,
    language text default 'en',
    created_at timestamptz default now()
);
create table if not exists public.analysis_results (
    id uuid primary key default gen_random_uuid(),
    document_id uuid references public.documents(id) on delete cascade,
    analysis_type text not null check (analysis_type in ('risk', 'compliance', 'clause', 'summary')),
    risk_level text check (risk_level in ('low', 'medium', 'high', 'critical')),
    issues_found integer default 0,
    results jsonb default '{}',
    created_at timestamptz default now()
);
create table if not exists public.clauses (
    id uuid primary key default gen_random_uuid(),
    document_id uuid references public.documents(id) on delete cascade,
    clause_type text not null,
    clause_text text not null,
    risk_score double precision,
    position_start integer,
    position_end integer
);
create index idx_documents_user on public.documents(user_id);
create index idx_analysis_doc on public.analysis_results(document_id);
create index idx_clauses_doc on public.clauses(document_id);
