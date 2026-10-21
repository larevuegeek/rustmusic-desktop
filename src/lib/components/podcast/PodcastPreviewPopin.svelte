<script lang="ts">
  // Aperçu d'une émission (clic droit / « … ») : la fiche à gauche, les derniers épisodes à droite.
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t, currentLocale } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { episodeDate, episodeDuration } from "#lib/helper/tools/podcastFormat";
  import { isWebUrl } from "#lib/helper/tools/safeHtml";
  import SafeHtml from "#lib/components/ui/text/SafeHtml.svelte";
  import PodcastCover from "./PodcastCover.svelte";
  import type { DirectoryResult, PodcastPreview } from "#lib/types/ui/podcast/Podcast";

  let { podcast }: { podcast: DirectoryResult } = $props();

  let apercu = $state<PodcastPreview | null>(null);
  let erreur = $state(false);
  $effect(() => {
    const url = podcast.feed_url;
    apercu = null;
    erreur = false;
    podcastStore.preview(url).then(
      (p) => { if (url === podcast.feed_url) apercu = p; },
      () => { if (url === podcast.feed_url) erreur = true; },
    );
  });

  const suivi = $derived($podcastStore.podcasts.find((p) => p.feed_url === podcast.feed_url) ?? null);
  // Abonné : la pochette locale ; l'image distante n'est permise que de quelques hôtes (CSP).
  const locale = $derived(suivi?.cover_path ?? null);
  const image = $derived(locale ? null : (podcast.image_url ?? apercu?.image_url ?? null));
  const formatNumber = (n: number) => n.toLocaleString($currentLocale);
  const derniere = $derived(apercu?.episodes[0]?.published_at ?? podcast.released_at ?? null);
  let descriptionOuverte = $state(false);

  let ajout = $state(false);
  async function follow(): Promise<number | null> {
    if (suivi) return suivi.id;
    ajout = true;
    try {
      const p = await podcastStore.follow(podcast.feed_url);
      toasts.push({ type: "success", title: $t("podcasts.followed").replace("{title}", p.title) });
      return p.id;
    } catch (err) {
      toasts.push({ type: "error", title: $t("podcasts.error_follow"), message: String(err) });
      return null;
    } finally {
      ajout = false;
    }
  }

  // La page de l'émission : ses épisodes et la lecture (abonnement à la clé, comme un clic sur la carte).
  async function openPage() {
    const id = await follow();
    if (id == null) return;
    popinStore.close();
    goto(`/podcasts/${id}`);
  }

  const pastille = "h-7 px-2.5 inline-flex items-center gap-1.5 rounded-full text-[12px] font-semibold bg-black/5 dark:bg-white/8 text-(--rg-tx2)";
</script>

<div class="relative grid grid-cols-[minmax(0,19rem)_minmax(0,1fr)] md:h-[min(84vh,640px)] max-md:grid-cols-1 max-md:max-h-[86vh] max-md:overflow-y-auto">
  <button type="button" onclick={() => popinStore.close()} aria-label={$t("common.close")} title={$t("common.close")}
          class="absolute right-3 top-3 z-20 w-8.5 h-8.5 rounded-full flex items-center justify-center cursor-pointer transition-colors
                 bg-black/30 text-white hover:bg-black/50 backdrop-blur-sm">
    <Icon icon="material-symbols:close-rounded" width="20" />
  </button>

  <!-- La fiche, sur le reflet flouté de sa pochette. -->
  <!-- Fond fixe ; seul le contenu défile s'il le faut (description dépliée), sans barre. -->
  <aside class="relative overflow-hidden min-h-0 border-r border-(--rg-line) max-md:border-r-0 max-md:border-b">
    {#if locale}
      <PodcastCover path={locale} size="1x" class="absolute! inset-0 w-full h-full scale-150 blur-3xl opacity-45 dark:opacity-35" />
    {:else if image}
      <img src={image} alt="" aria-hidden="true" class="absolute inset-0 w-full h-full object-cover scale-150 blur-3xl opacity-45 dark:opacity-35" />
    {/if}
    <div class="absolute inset-0 bg-linear-to-b from-transparent via-(--rg-carte)/60 to-(--rg-carte)"></div>

    <div class="relative h-full overflow-y-auto scrollbar-none flex flex-col p-6 pt-7">
      <span class="block shrink-0 w-36 max-md:w-28 aspect-square self-center overflow-hidden rounded-2xl bg-(--rg-s2) shadow-[0_18px_44px_rgba(0,0,0,0.35)]">
        {#if image}
          <img src={image} alt="" class="w-full h-full object-cover" />
        {:else}
          <PodcastCover path={locale} icone={48} class="w-full h-full" />
        {/if}
      </span>

      <p class="mt-4 text-[11.5px] font-bold uppercase tracking-widest text-violet-600 dark:text-violet-300">
        {[$t("podcasts.kicker_podcast"), podcast.genre].filter(Boolean).join(" · ")}
      </p>
      <h2 class="mt-1 text-[22px] font-extrabold leading-tight tracking-[-0.01em] text-(--rg-tx) line-clamp-3">{podcast.title}</h2>
      {#if podcast.author}<p class="mt-1 text-[14px] font-semibold text-(--rg-tx2) truncate">{podcast.author}</p>{/if}

      <div class="mt-3 flex flex-wrap gap-1.5">
        {#if apercu}
          <span class={pastille}><Icon icon="material-symbols:podcasts-rounded" width="14" />{formatNumber(apercu.episode_count)} {$t(apercu.episode_count === 1 ? "podcasts.episodes_one" : "podcasts.episodes_n")}</span>
        {/if}
        {#if derniere}
          <span class={pastille}><Icon icon="material-symbols:schedule-outline-rounded" width="14" />{episodeDate(derniere, $currentLocale)}</span>
        {/if}
      </div>

      <div class="mt-4 flex flex-col gap-2">
        {#if suivi}
          <span class="h-10.5 inline-flex items-center justify-center gap-1.5 rounded-full text-[13.5px] font-bold border border-(--rg-bd2) text-(--rg-gtx)">
            <Icon icon="material-symbols:check-rounded" width="18" />{$t("podcasts.following")}
          </span>
        {:else}
          <button type="button" onclick={follow} disabled={ajout}
                  class="h-10.5 inline-flex items-center justify-center gap-2 rounded-full cursor-pointer text-[13.5px] font-bold transition
                         bg-violet-600 text-white hover:bg-violet-500 shadow-[0_8px_22px_rgba(139,92,246,0.35)] disabled:opacity-70 disabled:cursor-wait">
            <Icon icon={ajout ? "material-symbols:progress-activity" : "material-symbols:add-rounded"} width="19" class={ajout ? "animate-spin" : ""} />
            {$t("podcasts.follow")}
          </button>
        {/if}
        <div class="flex gap-2">
          <button type="button" onclick={openPage} disabled={ajout}
                  class="flex-1 h-10 inline-flex items-center justify-center gap-1.5 rounded-full border cursor-pointer text-[13px] font-bold transition-colors
                         border-(--rg-bd2) text-(--rg-tx) hover:bg-(--rg-s2) disabled:cursor-wait">
            {$t("podcasts.open_page")}<Icon icon="material-symbols:arrow-forward-rounded" width="17" />
          </button>
          {#if isWebUrl(apercu?.link)}
            <a href={apercu?.link} title={$t("podcasts.website")} aria-label={$t("podcasts.website")}
               class="w-10 h-10 shrink-0 rounded-full flex items-center justify-center transition-colors border border-(--rg-bd2) text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
              <Icon icon="material-symbols:language" width="19" />
            </a>
          {/if}
        </div>
      </div>

      {#if apercu?.description}
        <SafeHtml html={apercu.description} class="mt-4 text-[13.5px] leading-relaxed text-(--rg-tx2) {descriptionOuverte ? '' : 'line-clamp-3'}" />
        <button type="button" onclick={() => (descriptionOuverte = !descriptionOuverte)}
                class="mt-1 self-start text-[12.5px] font-semibold cursor-pointer text-(--rg-tx) hover:underline underline-offset-2">
          {descriptionOuverte ? $t("podcasts.show_less") : $t("podcasts.show_more")}
        </button>
      {/if}
    </div>
  </aside>

  <!-- Les derniers épisodes. -->
  <section class="min-h-0 overflow-y-auto scrollbar-app px-6 pt-7 pb-6">
    <h3 class="mb-3 pr-10 text-[12px] font-bold uppercase tracking-widest text-(--rg-mu)">{$t("podcasts.preview_latest")}</h3>
    {#if erreur}
      <p class="py-6 text-sm text-(--rg-mu)">{$t("podcasts.preview_error")}</p>
    {:else if !apercu}
      <div class="flex flex-col gap-3" aria-busy="true">
        {#each { length: 6 } as _, i (i)}
          <div class="flex gap-3.5 py-2">
            <span class="w-12 h-12 shrink-0 rounded-xl bg-(--rg-s2) animate-pulse"></span>
            <span class="flex-1 flex flex-col gap-2 pt-1">
              <span class="h-3.5 rounded bg-(--rg-s2) animate-pulse" style="width: {85 - i * 7}%"></span>
              <span class="h-3 w-1/2 rounded bg-(--rg-s2) animate-pulse"></span>
            </span>
          </div>
        {/each}
      </div>
    {:else}
      <ul class="flex flex-col">
        {#each apercu.episodes as e, i (i)}
          {@const d = e.published_at ? new Date(e.published_at.replace(" ", "T") + "Z") : null}
          <li class="flex gap-3.5 py-3 border-b border-(--rg-line) last:border-b-0">
            <span class="w-12 h-12 shrink-0 rounded-xl flex flex-col items-center justify-center leading-none bg-violet-500/10 text-violet-700 dark:text-violet-200">
              {#if d && !Number.isNaN(d.getTime())}
                <span class="text-[17px] font-extrabold tabular-nums">{d.toLocaleDateString($currentLocale, { day: "numeric", timeZone: "UTC" })}</span>
                <span class="mt-0.5 text-[10px] font-bold uppercase tracking-wide">{d.toLocaleDateString($currentLocale, { month: "short", timeZone: "UTC" }).replace(".", "")}</span>
              {:else}
                <Icon icon="material-symbols:podcasts-rounded" width="20" />
              {/if}
            </span>
            <div class="min-w-0 flex-1">
              <p class="text-[14px] font-semibold leading-snug text-(--rg-tx) line-clamp-2">{e.title}</p>
              {#if e.summary}<p class="mt-0.5 text-[12.5px] leading-relaxed text-(--rg-mu) line-clamp-2">{e.summary}</p>{/if}
              {#if e.duration}
                <p class="mt-1 inline-flex items-center gap-1 text-[12px] font-semibold text-(--rg-tx2)">
                  <Icon icon="material-symbols:schedule-outline-rounded" width="13" />{episodeDuration(e.duration)}
                </p>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>
