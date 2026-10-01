// Appariement retenu et valeurs cochées, par fichier et par champ.
import { untrack } from "svelte";
import { messageErreur } from "$lib/helper/tools/errorTools";
import { valueOf, type WorkshopFile } from "$lib/stores/tags/tagWorkshop.store";
import {
  prepareImageFromUrl,
  type DownloadedImage,
} from "$lib/services/tags/tagEditor.service";
import {
  proposedValues,
  SOURCE_FIELDS,
  type AlbumTrack,
  type MatchProposal,
  type SourceField,
  type TrackMatch,
} from "$lib/services/tags/metadata.service";

// La pochette se désigne comme un champ sans en être un (écrite via `ImagePlan::SetCover`).
export const COVER = "cover";
export type CellField = SourceField | typeof COVER;

export const cellKey = (path: string, field: CellField) => `${path} ${field}`;
export const coverKey = (path: string) => cellKey(path, COVER);
export const shortName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Un champ d'un morceau, avec sa valeur d'ici et celle de là-bas. */
export type Cell = {
  field: SourceField;
  from: string;
  to: string;
  /** `absent` : la source ne dit rien. `same` : elle dit la même chose. */
  state: "differs" | "same" | "absent";
};

/** Un fichier et la piste distante à laquelle il a été apparié. */
export type Pair = {
  index: number;
  file: WorkshopFile;
  track: AlbumTrack;
  /** Douteux : l'appariement mérite un coup d'œil avant d'être suivi. */
  doubtful: boolean;
  cells: Cell[];
  diffs: Cell[];
};

export type FieldSummary = {
  field: CellField;
  label: string;
  total: number;
  picked: number;
};

export class SourceSelection {
  proposal: MatchProposal | null = $state(null);
  /** Seul état de décision : cocher un morceau ou un champ en coche plusieurs. */
  chosen = $state(new Set<string>());
  excluded = $state(new Set<number>());
  /** Morceaux dont le détail est ouvert. */
  opened = $state(new Set<number>());

  /** La pochette de l'album, téléchargée et apprêtée. */
  remoteCover: DownloadedImage | null = $state(null);
  coverLoading = $state(false);
  coverError: string | null = $state(null);

  readonly #getTargets: () => WorkshopFile[];

  /** À instancier pendant l'initialisation du composant (porte deux `$effect`). */
  constructor(getTargets: () => WorkshopFile[]) {
    this.#getTargets = getTargets;

    // Cochage d'ouverture : les cases vides seulement ; écraser reste une décision.
    $effect(() => {
      const current = this.proposal;
      untrack(() => {
        if (!current) {
          this.chosen = new Set();
          return;
        }
        const next = new Set<string>();
        for (const pair of this.pairs) {
          for (const cell of pair.diffs) {
            if (cell.from === "") next.add(cellKey(pair.file.path, cell.field));
          }
        }
        this.chosen = next;
      });
    });

    // Pochette téléchargée dès l'album retenu : son poids réel fait partie de la décision.
    $effect(() => {
      const url = this.proposal?.album.cover ?? null;
      untrack(() => {
        this.remoteCover = null;
        this.coverError = null;
        if (!url) return;

        this.coverLoading = true;
        prepareImageFromUrl(url)
          .then((img) => {
            this.remoteCover = img;
            // Cochée d'office sur les seuls fichiers qui n'en ont aucune.
            const next = new Set(this.chosen);
            for (const pair of this.pairs) {
              if (!pair.file.cover) next.add(coverKey(pair.file.path));
            }
            this.chosen = next;
          })
          .catch((e) => (this.coverError = messageErreur(e)))
          .finally(() => (this.coverLoading = false));
      });
    });
  }

  get targets(): WorkshopFile[] {
    return this.#getTargets();
  }

  matches: TrackMatch[] = $derived.by(() => this.proposal?.matches ?? []);
  /** Ce qui n'a pas trouvé sa piste, ou a été mis de côté à la main. */
  unmatched = $derived(
    this.matches.filter(
      (m) =>
        m.confidence === "rejected" ||
        m.remote_position === null ||
        this.excluded.has(m.local_index),
    ),
  );

  remoteTrack(position: number | null): AlbumTrack | null {
    if (position === null || !this.proposal) return null;
    return this.proposal.album.tracks.find((tk) => tk.position === position) ?? null;
  }

  localFile(index: number): WorkshopFile | null {
    return this.targets[index] ?? null;
  }

  pairs: Pair[] = $derived.by(() => {
    const album = this.proposal?.album;
    if (!album) return [];

    const out: Pair[] = [];
    for (const match of this.matches) {
      if (match.confidence === "rejected" || this.excluded.has(match.local_index)) continue;
      const file = this.localFile(match.local_index);
      const track = this.remoteTrack(match.remote_position);
      if (!file || !track) continue;

      const values = proposedValues(album, track);
      const cells: Cell[] = SOURCE_FIELDS.map((field) => {
        const to = values[field] ?? "";
        const from = valueOf(file, field).trim();
        return {
          field,
          from,
          to,
          // Absent n'est pas vide : proposer du vide effacerait la valeur du fichier.
          state: !to ? "absent" : to === from ? "same" : "differs",
        } as Cell;
      });

      out.push({
        index: match.local_index,
        file,
        track,
        doubtful: match.confidence === "doubtful",
        cells,
        diffs: cells.filter((c) => c.state === "differs"),
      });
    }
    return out;
  });

  /** La source propose-t-elle une pochette exploitable ? */
  hasCover = $derived(this.remoteCover !== null);
  /** Les fichiers qui recevront la pochette. */
  coverPicked: Pair[] = $derived(
    this.hasCover
      ? this.pairs.filter((pair) => this.chosen.has(coverKey(pair.file.path)))
      : [],
  );
  /** Parmi eux, ceux dont la pochette actuelle sera écrasée. */
  coversReplaced = $derived(this.coverPicked.filter((pair) => pair.file.cover).length);

  /** Toutes les valeurs modifiables, tous morceaux confondus. */
  allDiffs = $derived(
    this.pairs.flatMap((pair) => pair.diffs.map((cell) => ({ pair, cell }))),
  );
  picked = $derived(
    this.allDiffs.filter(({ pair, cell }) =>
      this.chosen.has(cellKey(pair.file.path, cell.field)),
    ),
  );
  changeCount = $derived(this.picked.length + this.coverPicked.length);
  fileCount = $derived(
    new Set([
      ...this.picked.map(({ pair }) => pair.file.path),
      ...this.coverPicked.map((pair) => pair.file.path),
    ]).size,
  );

  /** Les champs qui ont au moins une divergence, avec leur état de sélection. */
  fieldSummary: FieldSummary[] = $derived(
    [
      ...SOURCE_FIELDS.map((field) => {
        const cells = this.allDiffs.filter(({ cell }) => cell.field === field);
        return {
          field: field as CellField,
          label: `tags.${field}`,
          total: cells.length,
          picked: cells.filter(({ pair }) => this.chosen.has(cellKey(pair.file.path, field)))
            .length,
        };
      }),
      {
        field: COVER as CellField,
        label: "tags.cover",
        total: this.hasCover ? this.pairs.length : 0,
        picked: this.coverPicked.length,
      },
    ].filter((f) => f.total > 0),
  );

  allOpen = $derived(this.pairs.length > 0 && this.opened.size === this.pairs.length);

  /** Nouvel album apparié : exclusions et détails ouverts repartent de zéro. */
  load(proposal: MatchProposal) {
    this.proposal = proposal;
    this.excluded = new Set();
    this.opened = new Set();
  }

  toggleCell(path: string, field: SourceField) {
    const next = new Set(this.chosen);
    const key = cellKey(path, field);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    this.chosen = next;
  }

  /** Bascule un ensemble de valeurs : tout décocher si tout était coché. */
  toggleKeys(keys: string[]) {
    if (keys.length === 0) return;
    const next = new Set(this.chosen);
    if (keys.every((k) => next.has(k))) keys.forEach((k) => next.delete(k));
    else keys.forEach((k) => next.add(k));
    this.chosen = next;
  }

  /** Tout ce qu'un morceau peut recevoir : ses champs divergents et sa pochette. */
  pairKeys(pair: Pair): string[] {
    return [
      ...pair.diffs.map((c) => cellKey(pair.file.path, c.field)),
      ...(this.hasCover ? [coverKey(pair.file.path)] : []),
    ];
  }

  /** Coche ou décoche un champ sur toute la sélection. */
  toggleField(field: CellField) {
    this.toggleKeys(
      field === COVER
        ? this.pairs.map((pair) => coverKey(pair.file.path))
        : this.allDiffs
            .filter(({ cell }) => cell.field === field)
            .map(({ pair }) => cellKey(pair.file.path, field)),
    );
  }

  /** `all` : tout. `empty` : les trous seulement. `none` : rien. */
  selectAll(mode: "all" | "empty" | "none") {
    if (mode === "none") {
      this.chosen = new Set();
      return;
    }
    const next = new Set<string>();
    for (const { pair, cell } of this.allDiffs) {
      if (mode === "all" || cell.from === "") {
        next.add(cellKey(pair.file.path, cell.field));
      }
    }
    if (this.hasCover) {
      // « Les vides » vaut aussi pour l'image : seuls les fichiers sans pochette.
      for (const pair of this.pairs) {
        if (mode === "all" || !pair.file.cover) next.add(coverKey(pair.file.path));
      }
    }
    this.chosen = next;
  }

  toggleOpen(index: number) {
    const next = new Set(this.opened);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    this.opened = next;
  }

  toggleAllOpen() {
    this.opened = this.allOpen ? new Set() : new Set(this.pairs.map((p) => p.index));
  }

  toggleExcluded(index: number) {
    const next = new Set(this.excluded);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    this.excluded = next;
  }
}
