import { writable, derived, get } from "svelte/store";
import type { TrackLike } from "$lib/helper/tools/queueTools";

/**
 * La sélection multiple, partagée par toute l'application.
 *
 * # Tout est ramené à des pistes, tout de suite
 * On peut sélectionner un album, un artiste, un genre ou un dossier. Ces
 * objets pourraient être retenus tels quels et développés au moment d'agir —
 * mais le compteur afficherait alors « 3 éléments » sans dire combien de
 * morceaux, chaque action devrait savoir les développer, et deux d'entre elles
 * finiraient par le faire différemment.
 *
 * Un groupe est donc développé **au clic** : ses pistes entrent dans la
 * sélection comme les autres. Le décompte est honnête, et tout ce qui agit sur
 * la sélection continue de ne connaître que des pistes.
 *
 * `groupes` retient ce qu'un groupe a apporté, pour savoir le décocher et pour
 * afficher son état.
 */

type SelectionState = {
  active: boolean;
  /** Pistes sélectionnées, par identifiant. */
  tracks: Map<string, TrackLike>;
  /** Groupes cochés → identifiants des pistes qu'ils ont apportées. */
  groupes: Map<string, string[]>;
  /**
   * Ordre de la liste affichée, pour la sélection par plage.
   *
   * Renseigné par la vue courante : le magasin ne peut pas le deviner, et deux
   * vues du même contenu ne le rangent pas pareil.
   */
  ordre: string[];
  /** Dernier élément cliqué, point de départ d'une plage. */
  ancre: string | null;
};

const state = writable<SelectionState>({
  active: false,
  tracks: new Map(),
  groupes: new Map(),
  ordre: [],
  ancre: null,
});

/** Les pistes de `ordre`, telles que la vue les a déclarées. */
let catalogue: Map<string, TrackLike> = new Map();

export const selectionStore = {
  subscribe: derived(state, ($s) => ({
    active: $s.active,
    tracks: $s.tracks,
    count: $s.tracks.size,
    ids: new Set($s.tracks.keys()),
    groupes: new Set($s.groupes.keys()),
    ancre: $s.ancre,
  })).subscribe,

  /** Entre en mode sélection. */
  start() {
    state.update((s) => ({ ...s, active: true, tracks: new Map(), groupes: new Map(), ancre: null }));
  },

  /** Sort du mode sélection et vide tout. */
  stop() {
    state.set({ active: false, tracks: new Map(), groupes: new Map(), ordre: [], ancre: null });
  },

  /**
   * Déclare l'ordre de la liste affichée.
   *
   * À appeler par la vue quand son contenu ou son tri change. Sans cela, la
   * sélection par plage n'a aucun sens : « de celui-ci à celui-là » suppose un
   * entre-deux, et lui seul le connaît.
   */
  setOrder(items: { id: string; track: TrackLike }[]) {
    catalogue = new Map(items.map((i) => [i.id, i.track]));
    state.update((s) => ({ ...s, ordre: items.map((i) => i.id) }));
  },

  /** Coche ou décoche une piste, et en fait l'ancre d'une future plage. */
  toggle(id: string, track: TrackLike) {
    state.update((s) => {
      const next = new Map(s.tracks);
      if (next.has(id)) next.delete(id);
      else next.set(id, track);
      return { ...s, tracks: next, ancre: id };
    });
  },

  /**
   * Étend la sélection de l'ancre jusqu'à `id`.
   *
   * Sans ancre — premier clic avec Maj — se comporte comme un clic simple : il
   * faut bien un point de départ, et refuser silencieusement laisserait croire
   * à une panne.
   */
  selectRange(id: string) {
    const s = get(state);
    if (!s.ancre || s.ordre.length === 0) {
      const track = catalogue.get(id);
      if (track) selectionStore.toggle(id, track);
      return;
    }

    const depart = s.ordre.indexOf(s.ancre);
    const arrivee = s.ordre.indexOf(id);
    if (depart < 0 || arrivee < 0) return;

    const [bas, haut] = depart <= arrivee ? [depart, arrivee] : [arrivee, depart];

    state.update((st) => {
      const next = new Map(st.tracks);
      for (const cle of st.ordre.slice(bas, haut + 1)) {
        const track = catalogue.get(cle);
        if (track) next.set(cle, track);
      }
      // L'ancre ne bouge pas : étendre puis réduire une plage doit partir du
      // même point, comme dans un explorateur de fichiers.
      return { ...st, tracks: next };
    });
  },

  /**
   * Coche ou décoche un groupe — album, artiste, genre, dossier — avec toutes
   * ses pistes.
   */
  toggleGroup(cle: string, items: { id: string; track: TrackLike }[]) {
    state.update((s) => {
      const groupes = new Map(s.groupes);
      const tracks = new Map(s.tracks);

      const apportees = groupes.get(cle);
      if (apportees) {
        // On ne retire que ce que ce groupe avait apporté : une piste cochée à
        // la main avant lui doit survivre à son décochage.
        for (const id of apportees) tracks.delete(id);
        groupes.delete(cle);
      } else {
        const ajoutees: string[] = [];
        for (const item of items) {
          if (!tracks.has(item.id)) ajoutees.push(item.id);
          tracks.set(item.id, item.track);
        }
        groupes.set(cle, ajoutees);
      }

      return { ...s, tracks, groupes };
    });
  },

  has(id: string): boolean {
    return get(state).tracks.has(id);
  },

  hasGroup(cle: string): boolean {
    return get(state).groupes.has(cle);
  },

  /**
   * Coche tout ce que la vue courante affiche.
   *
   * S'appuie sur l'ordre déclaré plutôt que sur une liste passée en propriété :
   * celle-ci n'était renseignée que par l'onglet Morceaux, si bien que « Tout »
   * ne faisait rien sur les autres — le mode sélection semblait cassé alors que
   * seul son inventaire manquait.
   */
  selectAllVisible() {
    state.update((s) => {
      const next = new Map(s.tracks);
      for (const id of s.ordre) {
        const track = catalogue.get(id);
        if (track) next.set(id, track);
      }
      return { ...s, tracks: next };
    });
  },

  /** Coche tout ce qu'une liste contient. */
  selectAll(items: { id: string; track: TrackLike }[]) {
    state.update((s) => {
      const next = new Map(s.tracks);
      for (const item of items) next.set(item.id, item.track);
      return { ...s, tracks: next };
    });
  },

  deselectAll() {
    state.update((s) => ({ ...s, tracks: new Map(), groupes: new Map(), ancre: null }));
  },

  getSelectedTracks(): TrackLike[] {
    return Array.from(get(state).tracks.values());
  },

  /**
   * Sur quoi porte une action déclenchée depuis `id`.
   *
   * Un clic droit sur une ligne **déjà cochée** agit sur toute la sélection :
   * c'est ce qu'on attend après en avoir coché vingt. Ailleurs, il agit sur
   * cette seule ligne.
   *
   * Ne modifie rien. Activer le mode sélection au premier clic droit ferait
   * surgir des cases à cocher que personne n'a demandées, et changerait le sens
   * du clic suivant.
   */
  targetsFor(id: string, track: TrackLike): TrackLike[] {
    const s = get(state);
    if (s.active && s.tracks.has(id)) return Array.from(s.tracks.values());
    return [track];
  },
};
