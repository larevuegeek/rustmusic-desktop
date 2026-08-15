<script lang="ts">
  // Recherche d'une piste sur Deezer, pour un seul fichier.
  //
  // # Ce que ce composant ne fait pas
  // Il ne décide rien et n'écrit nulle part : il rend les valeurs trouvées à
  // son appelant, qui les verse dans son formulaire. C'est là, champ par champ,
  // que l'utilisateur voit ce qui change et peut revenir en arrière.
  //
  // # Pourquoi chercher une piste et non un album
  // Pour un fichier isolé, passer par l'album demande deux décisions — quel
  // album, puis quelle piste — quand le titre en suffit d'une.
  import Icon from "@iconify/svelte";
  import { untrack } from "svelte";
  import { t } from "$lib/i18n";
  import {
    formatDuration,
    searchTracks,
    trackValues,
    type TrackHit,
    type TrackValues,
  } from "$lib/services/tags/metadata.service";

  const {
    initialQuery = "",
    onpick,
    oncancel,
  }: {
    /** Amorce de recherche, devinée par l'appelant depuis les tags existants. */
    initialQuery?: string;
    onpick: (values: TrackValues, hit: TrackHit) => void;
    oncancel: () => void;
  } = $props();

  // Le composant est monté à neuf à chaque ouverture : l'amorce ne sert qu'au
  // départ. La lire sans la suivre évite qu'une frappe dans le formulaire
  // derrière ne réécrive la recherche en cours.
  let query = $state(untrack(() => initialQuery));
  let hits: TrackHit[] = $state([]);
  let searching = $state(false);
  /** Identifiant de la piste en cours de lecture, pour n'animer qu'elle. */
  let loadingId: number | null = $state(null);
  let error: string | null = $state(null);
  let searched = $state(false);
  let input: HTMLInputElement | null = $state(null);

  /** Garde de démarrage : la recherche d'ouverture ne doit partir qu'une fois. */
  let started = false;

  // La recherche est le seul geste possible à l'ouverture : demander un clic
  // dans le champ avant de pouvoir taper serait une friction gratuite. Et une
  // amorce déjà juste est le cas courant — on cherche d'emblée plutôt que
  // d'attendre un clic qui ne dirait rien de plus.
  //
  // Tout est dans `untrack` : `run()` lit et écrit `query` et `searching`, ce
  // qui relancerait l'effet à chaque fin de recherche — donc en boucle.
  $effect(() => {
    input?.focus();
    input?.select();
    untrack(() => {
      if (started || !query.trim()) return;
      started = true;
      run();
    });
  });

  async function run() {
    if (!query.trim() || searching) return;
    searching = true;
    error = null;
    try {
      hits = await searchTracks(query, 12);
      searched = true;
    } catch (e) {
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      searching = false;
    }
  }

  async function pick(hit: TrackHit) {
    loadingId = hit.id;
    error = null;
    try {
      onpick(await trackValues(hit.id), hit);
    } catch (e) {
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      loadingId = null;
    }
  }
</script>

<div class="flex flex-col min-h-0 h-full">
  <div class="shrink-0 flex gap-1.5 px-4 py-3">
    <div class="relative flex-1 min-w-0">
      <Icon
        icon="lucide:search"
        width="13"
        class="absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none
               text-neutral-400 dark:text-neutral-500"
      />
      <input
        bind:this={input}
        type="text"
        bind:value={query}
        onkeydown={(e) => {
          if (e.key === "Enter") run();
          // La recherche est une couche au-dessus du formulaire : Échap doit
          // la refermer, pas fermer l'éditeur et perdre la saisie.
          if (e.key === "Escape") {
            e.stopPropagation();
            oncancel();
          }
        }}
        placeholder={$t("source.track_placeholder")}
        class="w-full h-8 pl-7.5 pr-2.5 rounded-lg text-[12px] outline-none
               bg-white dark:bg-white/5
               ring-1 ring-inset ring-neutral-200 dark:ring-white/10
               focus:ring-emerald-500/60
               text-neutral-900 dark:text-neutral-100
               placeholder:text-neutral-400 dark:placeholder:text-neutral-600"
      />
    </div>
    <button
      type="button"
      onclick={run}
      disabled={searching || !query.trim()}
      class="shrink-0 px-3 h-8 rounded-lg flex items-center gap-1.5 text-[12px] font-medium
             cursor-pointer transition-colors disabled:opacity-40
             bg-emerald-500 text-white hover:bg-emerald-600"
    >
      {#if searching}
        <Icon icon="lucide:loader-circle" width="12" class="animate-spin" />
      {/if}
      {$t("source.search")}
    </button>
  </div>

  {#if error}
    <p class="shrink-0 mx-4 mb-2 flex items-start gap-1.5 px-2.5 py-2 rounded-lg
              text-[11px] bg-red-500/10 text-red-500">
      <Icon icon="lucide:alert-triangle" width="12" class="shrink-0 mt-0.5" />
      <span class="min-w-0">{error}</span>
    </p>
  {/if}

  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-2 pb-3 space-y-0.5">
    {#each hits as hit (hit.id)}
      <button
        type="button"
        onclick={() => pick(hit)}
        disabled={loadingId !== null}
        class="w-full flex items-center gap-2.5 p-1.5 rounded-lg text-left
               cursor-pointer transition-colors disabled:opacity-50
               hover:bg-neutral-200/60 dark:hover:bg-white/6"
      >
        <span class="shrink-0 w-10 h-10 rounded overflow-hidden
                     bg-neutral-200 dark:bg-white/5">
          {#if hit.cover}
            <img src={hit.cover} alt="" class="w-full h-full object-cover" />
          {/if}
        </span>
        <span class="min-w-0 flex-1">
          <span class="block text-[12px] font-medium truncate
                       text-neutral-800 dark:text-neutral-100">
            {hit.title}
          </span>
          <!-- La durée départage deux versions d'un même titre — studio,
               live, remix — mieux que n'importe quelle autre donnée. -->
          <span class="block text-[10.5px] truncate text-neutral-400 dark:text-neutral-500">
            {hit.artist} · {hit.album} · {formatDuration(hit.duration)}
          </span>
        </span>
        <Icon
          icon={loadingId === hit.id ? "lucide:loader-circle" : "lucide:chevron-right"}
          width="13"
          class="shrink-0 text-neutral-300 dark:text-neutral-600
                 {loadingId === hit.id ? 'animate-spin' : ''}"
        />
      </button>
    {:else}
      {#if searched && !searching}
        <p class="px-2 py-8 text-center text-[12px] text-neutral-400 dark:text-neutral-500">
          {$t("source.no_track_result")}
        </p>
      {/if}
    {/each}
  </div>
</div>
