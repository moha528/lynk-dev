-- Profils du Git Manager : une racine analysée, et les dépôts qu'on en retient.
--
-- Même choix que pour `dev_profiles` : la liste de chemins est courte, lue et
-- écrite en bloc, donc un tableau JSON plutôt qu'une table fille.
CREATE TABLE IF NOT EXISTS git_profiles (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    root_path   TEXT NOT NULL,
    repo_paths  TEXT NOT NULL DEFAULT '[]',
    -- Millisecondes depuis l'époque Unix, comme côté front.
    created_at  INTEGER NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
