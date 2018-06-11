CREATE TABLE IF NOT EXISTS "tags" (
    "id" INTEGER PRIMARY KEY AUTOINCREMENT,
    "name" TEXT NOT NULL UNIQUE,
    "approved" INTEGER NOT NULL DEFAULT 0
);

INSERT INTO tags (name, approved) VALUES
('UFO', 1),
('CABAL', 1)