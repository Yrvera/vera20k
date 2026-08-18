PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    system TEXT NOT NULL,
    subsystem TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    modified_time REAL NOT NULL,
    checksum TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY,
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    heading_path TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    text TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
    text,
    heading_path,
    path UNINDEXED,
    chunk_id UNINDEXED,
    tokenize = 'unicode61'
);

CREATE TABLE IF NOT EXISTS symbols (
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    symbol TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS addresses (
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    address TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ini_keys (
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    key TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS rust_paths (
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS links (
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    target TEXT NOT NULL,
    exists_flag INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY,
    source_document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    edge_kind TEXT NOT NULL,
    target TEXT NOT NULL,
    target_document_id INTEGER REFERENCES documents(id) ON DELETE CASCADE,
    source_start_line INTEGER,
    source_end_line INTEGER,
    weight REAL NOT NULL,
    evidence TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_documents_system ON documents(system);
CREATE INDEX IF NOT EXISTS idx_documents_source_kind ON documents(source_kind);
CREATE INDEX IF NOT EXISTS idx_symbols_symbol ON symbols(symbol);
CREATE INDEX IF NOT EXISTS idx_addresses_address ON addresses(address);
CREATE INDEX IF NOT EXISTS idx_ini_keys_key ON ini_keys(key);
CREATE INDEX IF NOT EXISTS idx_rust_paths_path ON rust_paths(path);
CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id);
CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_document_id);
CREATE INDEX IF NOT EXISTS idx_edges_kind_target ON edges(edge_kind, target);
CREATE INDEX IF NOT EXISTS idx_edges_target_doc ON edges(target_document_id);
