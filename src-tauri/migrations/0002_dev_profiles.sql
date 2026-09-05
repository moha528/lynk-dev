-- Profils du Dev Manager.
--
-- La version Electron les gardait dans un `dev-profiles.json` du dossier
-- utilisateur. Ils passent en base pour ne plus avoir deux mécanismes de
-- persistance concurrents dans l'application.
--
-- `services` est un tableau JSON plutôt qu'une table fille : la liste est
-- courte, toujours lue et écrite en bloc avec son profil, et jamais interrogée
-- ligne par ligne. Une table fille n'apporterait que des jointures.
CREATE TABLE IF NOT EXISTS dev_profiles (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    root_path   TEXT NOT NULL,
    services    TEXT NOT NULL DEFAULT '[]',
    -- Millisecondes depuis l'époque Unix, comme côté front.
    created_at  INTEGER NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
