/**
 * Aides à la sélection multiple dans une liste.
 *
 * Le point d'ancrage et l'ordre affiché vivent chez l'appelant : une liste
 * filtrée ou repliée n'a pas le même ordre que ses données, et c'est **l'ordre
 * à l'écran** qui doit gouverner une sélection par plage — sinon un Maj+clic
 * coche des lignes qu'on ne voit pas.
 */

/** Ce qu'une ligne transmet à l'écran quand on coche sa case. */
export type CheckOptions = {
  /** Maj enfoncée : l'appelant coche toute la plage depuis son ancre. */
  shiftKey: boolean;
  /** Identifiants **dans l'ordre affiché** — filtres et replis compris. */
  ordered: string[];
};

/**
 * Identifiants compris entre `from` et `to` dans `ordered`, bornes incluses,
 * quel que soit le sens du geste.
 *
 * Rend `[to]` seul si l'une des bornes a disparu de l'affichage — cocher une
 * plage arbitraire serait pire que ne rien faire.
 */
export function rangeBetween(ordered: string[], from: string, to: string): string[] {
  const start = ordered.indexOf(from);
  const end = ordered.indexOf(to);
  if (start === -1 || end === -1) return [to];
  const [low, high] = start <= end ? [start, end] : [end, start];
  return ordered.slice(low, high + 1);
}

/**
 * Voisin de `current` dans `ordered`, pour la navigation au clavier.
 *
 * La liste ne boucle pas : arrivé en bas, la flèche du bas ne fait plus rien.
 * Un retour au premier élément donne l'impression d'avoir perdu sa place.
 */
export function neighbour(
  ordered: string[],
  current: string | null,
  direction: 1 | -1,
): string | null {
  if (ordered.length === 0) return null;
  if (!current) return direction === 1 ? ordered[0] : ordered[ordered.length - 1];

  const index = ordered.indexOf(current);
  if (index === -1) return ordered[0];

  const next = index + direction;
  if (next < 0 || next >= ordered.length) return null;
  return ordered[next];
}
