<script module lang="ts">
import type { TrackListView } from "$lib/types/ui/library/track/TrackListView";

// Un mix « du jour » ne se retire pas à chaque retour sur l'accueil.
const cache = new Map<string, Promise<TrackListView[]>>();
// TEMPORAIRE : deux mises en page à comparer.
const VARIANTE_MIX: "carrousel" | "horizontale" = "horizontale";
</script>

<script lang="ts">
import Icon from "@iconify/svelte";
import { untrack } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import { t } from "$lib/i18n";
import { handlePlayTrack } from "$lib/actions/player/PlayerAction";
import { versFileDAttente } from "$lib/mapper/queue/mapQueueTrack";
import { teinte } from "$lib/helper/tools/teinte";
import { dureeEcoute } from "$lib/helper/tools/dateTools";
import CoverImg from "$lib/components/ui/image/CoverImg.svelte";
import Carousel from "$lib/components/ui/carousel/Carousel.svelte";
import type { GenreView } from "$lib/types/ui/library/genre/GenreView";
import type { AlbumListView } from "$lib/types/ui/library/album/AlbumListView";

let { libraryId, genres, albums = [], hires }: { libraryId: number; genres: GenreView[]; albums?: AlbumListView[]; hires: number } = $props();

const TAILLE_MIX = 50;
// En dessous, la décennie ne fait pas un mix.
const MIN_DECENNIE = 10;

type Spec = { cle: string; type: string; nom: string; aide: string; h: number; args: Record<string, unknown>; pre?: string; court?: string };

const specs = $derived.by((): Spec[] => {
  const liste: Spec[] = genres.slice(0, 2).map((g) => ({
    cle: `genre:${g.name}`, type: $t('home.mix_of_day'), nom: g.name, aide: g.name, h: teinte(g.name),
    args: { kind: "genre", genre: g.name },
  }));
  // Libellés courts sur la pochette, l'explication entière en info-bulle.
  liste.push({ cle: "oublies", type: $t('home.rediscover'), nom: $t('home.forgotten'),
               aide: $t('home.forgotten_kind'), h: 200, args: { kind: "oublies" } });
  if (hires > 0) liste.push({ cle: "hires", type: $t('home.studio_quality'), nom: "Hi-Res",
                              aide: $t('home.hires_kind'), h: 150, args: { kind: "hires" } });
  return liste;
});

// Une décennie par tranche d'albums datés : « 80 » avant 2000, « 2010 » après.
const decennies = $derived.by((): Spec[] => {
  const pistesPar = new Map<number, number>();
  for (const a of albums) {
    if (!a.year || a.year < 1900) continue;
    const d = Math.floor(a.year / 10) * 10;
    pistesPar.set(d, (pistesPar.get(d) ?? 0) + (a.total_tracks ?? 0));
  }
  return [...pistesPar.entries()]
    .filter(([, n]) => n >= MIN_DECENNIE)
    .sort(([a], [b]) => a - b)
    .map(([d]) => {
      const court = d >= 1920 && d < 2000 ? String(d % 100) : String(d);
      return {
        cle: `decennie:${d}`, type: $t('home.decade_pre'), nom: $t('home.decade_name').replace('{d}', court),
        aide: $t('home.decade_kind').replace('{a}', String(d)).replace('{b}', String(d + 9)),
        h: (((d - 1950) / 10) * 47 + 360) % 360, args: { kind: "decennie", decennie: d },
        pre: $t('home.decade_pre'), court: $t('home.decade_short').replace('{d}', court),
      };
    });
});
const tous = $derived([...specs, ...decennies]);

// Les morceaux de chaque mix, tirés à l'affichage : ce qu'on voit est ce qui jouera.
let pistes = $state<Record<string, TrackListView[]>>({});
let generation = $state(0);
const signature = $derived(`${libraryId}|${tous.map((s) => s.cle).join(",")}|${generation}`);

$effect(() => {
  void signature;
  const id = libraryId;
  const jour = new Date().toDateString();
  // Les tirages d'hier ne resserviront plus.
  for (const cle of [...cache.keys()]) if (!cle.endsWith(`|${jour}`)) cache.delete(cle);
  for (const s of untrack(() => tous)) {
    const cle = `${id}|${s.cle}|${jour}`;
    let tirage = cache.get(cle);
    if (!tirage) {
      tirage = invoke<TrackListView[]>("get_mix_tracks", { libraryId: id, ...s.args, limit: TAILLE_MIX })
        .catch(() => []);
      cache.set(cle, tirage);
    }
    tirage.then((liste) => { pistes[s.cle] = liste; });
  }
});

function renouveler() {
  for (const cle of [...cache.keys()]) if (cle.startsWith(`${libraryId}|`)) cache.delete(cle);
  pistes = {};
  generation++;
}

type Apercu = { pochettes: string[]; description: string; meta: string };

function apercu(liste: TrackListView[]): Apercu {
  const pochettes = [...new Set(liste.map((p) => p.thumbnail_path).filter(Boolean) as string[])].slice(0, 4);

  const parArtiste = new Map<string, number>();
  for (const p of liste) if (p.artist) parArtiste.set(p.artist, (parArtiste.get(p.artist) ?? 0) + 1);
  const tete = [...parArtiste.entries()].sort((a, b) => b[1] - a[1]).map(([a]) => a);
  const noms = tete.slice(0, 3).join(", ");
  const description = tete.length > 3 ? $t('home.mix_artists_more').replace('{a}', noms) : noms;

  const secondes = liste.reduce((s, p) => s + (p.duration ?? 0), 0);
  const meta = $t('home.mix_meta').replace('{n}', String(liste.length)).replace('{d}', dureeEcoute(secondes));

  return { pochettes, description, meta };
}

let lance = $state<string | null>(null);

async function lancer(s: Spec) {
  const liste = pistes[s.cle];
  if (!liste?.length || lance) return;
  lance = s.cle;
  try {
    const file = versFileDAttente(liste);
    await handlePlayTrack(file[0].path, file);
  } finally {
    lance = null;
  }
}
</script>

{#snippet carte(s: Spec, horizontale: boolean)}
  {@const liste = pistes[s.cle]}
  {@const a = liste ? apercu(liste) : null}
  <button type="button" onclick={() => lancer(s)} disabled={!liste?.length} title={s.aide}
          class="group flex text-left cursor-pointer min-w-0 w-full disabled:cursor-default
                 {horizontale ? 'flex-row items-center gap-4 @4xl:flex-col @4xl:items-stretch @4xl:gap-3' : 'flex-col gap-3'}"
          style="--h: {s.h}">

    <!-- La pochette du mix : quatre albums qu'il contient -->
    <div class="relative aspect-square overflow-hidden shrink-0
                shadow-[0_12px_32px_rgba(0,0,0,0.28)] ring-1 ring-black/5 dark:ring-white/[0.07]
                transition-transform duration-300 group-hover:-translate-y-1
                {horizontale ? 'w-32 rounded-xl @4xl:w-full @4xl:rounded-2xl' : 'w-full rounded-2xl'}">
      {#if !a}
        <div class="absolute inset-0 bg-neutral-200 dark:bg-white/6 animate-pulse"></div>
      {:else if a.pochettes.length >= 4}
        <div class="absolute inset-0 grid grid-cols-2 grid-rows-2">
          {#each a.pochettes as p (p)}
            <CoverImg path={p} size="2x" class="w-full h-full object-cover" />
          {/each}
        </div>
      {:else if a.pochettes.length > 0}
        <CoverImg path={a.pochettes[0]} size="full" class="absolute inset-0 w-full h-full object-cover" />
      {:else}
        <div class="bande-teintee absolute inset-0"></div>
      {/if}

      <div class="mix-voile absolute inset-0 {horizontale ? 'hidden @4xl:block' : ''}"></div>

      <div class="absolute left-4 right-16 bottom-4 flex-col gap-0.5 {horizontale ? 'hidden @4xl:flex' : 'flex'}">
        <span class="mix-accent text-[11px] font-bold uppercase tracking-[0.1em] truncate">{s.type}</span>
        <span class="text-[22px] @6xl:text-[26px] font-extrabold tracking-[-0.02em] leading-[1.1] text-white line-clamp-2">
          {s.nom}
        </span>
      </div>

      <span class="absolute rounded-full bg-[#22c55e] text-(--rg-on-g)
                   flex items-center justify-center shadow-[0_8px_20px_rgba(0,0,0,0.4)]
                   transition-all duration-200
                   {horizontale ? 'right-2 bottom-2 w-9 h-9 @4xl:right-3.5 @4xl:bottom-3.5 @4xl:w-12 @4xl:h-12' : 'right-3.5 bottom-3.5 w-12 h-12'}
                   {lance === s.cle ? 'opacity-100' : 'opacity-0 translate-y-1.5 group-hover:opacity-100 group-hover:translate-y-0'}">
        <Icon icon={lance === s.cle ? "lucide:loader-circle" : "mynaui:play-solid"} width={20}
              class={lance === s.cle ? "animate-spin" : ""} />
      </span>
    </div>

    <!-- Ce qu'il contient ; en carte horizontale, le type et le nom passent ici. -->
    <div class="flex flex-col gap-1 px-0.5 min-w-0 flex-1">
      {#if horizontale}
        <span class="@4xl:hidden accent-teinte text-[11px] font-bold uppercase tracking-[0.1em] truncate">{s.type}</span>
        <span class="@4xl:hidden text-xl font-extrabold tracking-[-0.02em] leading-tight truncate text-neutral-900 dark:text-white">{s.nom}</span>
      {/if}
      {#if a}
        <p class="text-[14px] leading-snug text-neutral-700 dark:text-neutral-200 line-clamp-2">{a.description}</p>
        <p class="text-[12.5px] text-neutral-500 dark:text-[#9aa39e]">{a.meta}</p>
      {:else}
        <div class="h-3.5 w-4/5 rounded bg-neutral-200 dark:bg-white/6 animate-pulse"></div>
        <div class="h-3 w-2/5 rounded bg-neutral-200 dark:bg-white/6 animate-pulse"></div>
      {/if}
    </div>
  </button>
{/snippet}

<!-- Tuile de décennie : quatre pochettes de l'époque (vignettes 1x, assez pour ~88 px), le millésime en grand. -->
{#snippet carteDecennie(s: Spec)}
  {@const liste = pistes[s.cle]}
  {@const a = liste ? apercu(liste) : null}
  <button type="button" onclick={() => lancer(s)} disabled={!liste?.length} title={s.aide}
          class="group flex flex-col gap-2.5 w-full text-left cursor-pointer disabled:cursor-default" style="--h: {s.h}">
    <div class="relative aspect-square w-full rounded-2xl overflow-hidden
                shadow-[0_12px_32px_rgba(0,0,0,0.28)] ring-1 ring-black/5 dark:ring-white/[0.07]
                transition-transform duration-300 group-hover:-translate-y-1">
      {#if !a}
        <div class="absolute inset-0 bg-neutral-200 dark:bg-white/6 animate-pulse"></div>
      {:else if a.pochettes.length >= 4}
        <div class="absolute inset-0 grid grid-cols-2 grid-rows-2">
          {#each a.pochettes as p (p)}
            <CoverImg path={p} size="1x" class="w-full h-full object-cover" />
          {/each}
        </div>
      {:else if a.pochettes.length > 0}
        <CoverImg path={a.pochettes[0]} size="2x" class="absolute inset-0 w-full h-full object-cover" />
      {:else}
        <div class="bande-teintee absolute inset-0"></div>
      {/if}
      <div class="mix-voile absolute inset-0"></div>

      <div class="absolute left-3.5 bottom-3 flex flex-col">
        <span class="mix-accent text-[10.5px] font-bold uppercase tracking-[0.14em]">{s.pre}</span>
        <span class="text-[40px] font-black tracking-[-0.04em] leading-[0.95] text-white">{s.court}</span>
      </div>

      <span class="absolute right-3 bottom-3 w-10 h-10 rounded-full bg-[#22c55e] text-(--rg-on-g)
                   flex items-center justify-center shadow-[0_8px_20px_rgba(0,0,0,0.4)] transition-all duration-200
                   {lance === s.cle ? 'opacity-100' : 'opacity-0 translate-y-1.5 group-hover:opacity-100 group-hover:translate-y-0'}">
        <Icon icon={lance === s.cle ? "lucide:loader-circle" : "mynaui:play-solid"} width={18}
              class={lance === s.cle ? "animate-spin" : ""} />
      </span>
    </div>

    <div class="flex flex-col gap-0.5 px-0.5 min-w-0">
      {#if a}
        <p class="text-[13px] leading-snug text-neutral-700 dark:text-neutral-200 line-clamp-2">{a.description}</p>
        <p class="text-xs text-neutral-500 dark:text-[#9aa39e]">{a.meta}</p>
      {:else}
        <div class="h-3.5 w-4/5 rounded bg-neutral-200 dark:bg-white/6 animate-pulse"></div>
        <div class="h-3 w-2/5 rounded bg-neutral-200 dark:bg-white/6 animate-pulse"></div>
      {/if}
    </div>
  </button>
{/snippet}

{#if specs.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('home.for_you')}</h2>
    <div class="flex items-center gap-4">
      <span class="hidden @3xl:inline text-sm text-neutral-500 dark:text-[#9aa39e]">{$t('home.for_you_desc')}</span>
      <button type="button" onclick={renouveler}
              class="flex items-center gap-1.5 text-sm font-semibold text-emerald-600 dark:text-emerald-400
                     hover:text-emerald-500 dark:hover:text-emerald-300 cursor-pointer transition-colors">
        <Icon icon="lucide:refresh-cw" width={14} /> {$t('home.new_mixes')}
      </button>
    </div>
  </div>

  {#if VARIANTE_MIX === "carrousel"}
    <!-- Carrousel : la carte garde sa taille, la rangée défile quand la place manque. -->
    <div class="flex gap-4 @4xl:gap-5 overflow-x-auto snap-x snap-mandatory pb-2 [scrollbar-width:thin]">
      {#each specs as s (s.cle)}
        <div class="w-55 shrink-0 snap-start @4xl:w-auto @4xl:flex-1 @4xl:min-w-0">{@render carte(s, false)}</div>
      {/each}
    </div>
  {:else}
    <!-- Étroit : deux cartes horizontales par ligne ; large : quatre carrés. -->
    <div class="grid grid-cols-1 @xl:grid-cols-2 @4xl:grid-cols-4 gap-4 @4xl:gap-5">
      {#each specs as s (s.cle)}
        {@render carte(s, true)}
      {/each}
    </div>
  {/if}
</section>
{/if}

{#if decennies.length > 0}
<section class="flex flex-col gap-4">
  <div class="flex items-baseline justify-between gap-4">
    <h2 class="text-2xl font-bold tracking-tight text-neutral-900 dark:text-neutral-100">{$t('home.decades')}</h2>
    <span class="hidden @3xl:inline text-sm text-neutral-500 dark:text-[#9aa39e]">{$t('home.decades_desc')}</span>
  </div>
  <!-- Carrousel à flèches, calées au milieu des pochettes. -->
  <Carousel axe="90px" class="gap-4 @4xl:gap-5 pt-1.5 pb-2">
    {#each decennies as s (s.cle)}
      <div class="w-40 @4xl:w-44 shrink-0 snap-start">{@render carteDecennie(s)}</div>
    {/each}
  </Carousel>
</section>
{/if}
