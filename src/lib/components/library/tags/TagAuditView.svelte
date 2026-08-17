<script lang="ts">
  // Ce qu'il y a à corriger dans la bibliothèque.
  //
  // # Pourquoi c'est l'écran d'accueil de l'atelier
  // L'atelier ouvert sans sélection affichait « aucun fichier chargé » et un
  // conseil. C'était un cul-de-sac : pour s'en servir il fallait déjà savoir
  // quel album ouvrir, donc avoir repéré le problème ailleurs.
  //
  // Ici l'écran vide devient le point d'entrée : il dit ce qui cloche, et un
  // clic verse les fichiers concernés dans l'atelier. Le reste — corriger,
  // récupérer depuis Deezer, écrire — était déjà là.
  //
  // # Trier par ce qui empêche, pas par ce qui manque
  // Un morceau sans titre est introuvable ; un morceau sans année se retrouve
  // très bien. Les deux ne se valent pas et ne se rangent pas ensemble.
  import Icon from "@iconify/svelte";
  import { fade } from "svelte/transition";
  import { t } from "$lib/i18n";
  import {
    auditLibrary,
    SEVERITY_ORDER,
    type AuditGroup,
    type AuditReport,
    type Severity,
  } from "$lib/services/tags/audit.service";

  const {
    libraryId,
    onpick,
  }: {
    libraryId: number;
    /** Verse une catégorie dans l'atelier. */
    onpick: (group: AuditGroup, label: string) => void;
  } = $props();

  let report: AuditReport | null = $state(null);
  let loading = $state(false);
  let error: string | null = $state(null);

  // La bibliothèque est une prop : elle change quand on navigue d'une
  // bibliothèque à l'autre sans quitter l'atelier.
  $effect(() => {
    run(libraryId);
  });

  async function run(id: number) {
    loading = true;
    error = null;
    try {
      report = await auditLibrary(id);
    } catch (e) {
      error = String((e as any)?.message ?? e ?? "");
    } finally {
      loading = false;
    }
  }

  /** Les catégories, regroupées par ce qu'elles demandent comme geste. */
  let sections = $derived.by(() => {
    // Passage par une locale typée : la propriété n'est affectée que dans une
    // fonction asynchrone, ce qui suffit à faire perdre son type à
    // l'inférence, qui la ramène alors à `null`.
    const current: AuditReport | null = report;
    const groups: AuditGroup[] = current?.groups ?? [];
    return SEVERITY_ORDER.map((severity) => ({
      severity,
      groups: groups.filter((g) => g.severity === severity),
    })).filter((s) => s.groups.length > 0);
  });

  // Un même fichier compte dans plusieurs catégories — sans titre **et** sans
  // année. Additionner les compteurs mentirait ; on compte les fichiers
  // distincts.
  let total = $derived.by(() => {
    const current: AuditReport | null = report;
    const groups: AuditGroup[] = current?.groups ?? [];
    return new Set(groups.flatMap((g) => g.paths)).size;
  });

  const ACCENT: Record<Severity, string> = {
    essential: "text-red-500",
    coherence: "text-amber-600 dark:text-amber-400",
    duplicate: "text-violet-500 dark:text-violet-400",
    incomplete: "text-sky-600 dark:text-sky-400",
    blocked: "text-neutral-400 dark:text-neutral-500",
  };

  const ICON: Record<Severity, string> = {
    essential: "lucide:circle-alert",
    coherence: "lucide:git-compare-arrows",
    duplicate: "lucide:copy",
    incomplete: "lucide:circle-dashed",
    blocked: "lucide:lock",
  };
</script>

<div class="flex-1 min-h-0 overflow-y-auto scrollbar-none">
  <div class="max-w-4xl mx-auto px-6 py-8">
    <!-- ─── En-tête ─── -->
    <div class="flex items-start gap-3 mb-6">
      <div class="min-w-0 flex-1">
        <h2 class="text-[15px] font-medium text-neutral-800 dark:text-neutral-100">
          {$t("audit.title")}
        </h2>
        <p class="mt-0.5 text-[12px] text-neutral-400 dark:text-neutral-500">
          {#if loading}
            {$t("audit.scanning")}
          {:else if report}
            <!-- Le dénominateur compte autant que le numérateur : « 412 » seul
                 ne dit pas si la bibliothèque est en bon état. -->
            {total} / {report.scanned} {$t("audit.tracks_concerned")}
          {/if}
        </p>
      </div>

      <button
        type="button"
        onclick={() => run(libraryId)}
        disabled={loading}
        class="shrink-0 flex items-center gap-1.5 px-2.5 h-8 rounded-lg text-[12px]
               cursor-pointer transition-colors disabled:opacity-40
               text-neutral-500 dark:text-neutral-400
               hover:bg-neutral-100 dark:hover:bg-white/6"
      >
        <Icon icon="lucide:refresh-cw" width="12" class={loading ? "animate-spin" : ""} />
        {$t("audit.rescan")}
      </button>
    </div>

    {#if error}
      <p class="flex items-start gap-2 px-3 py-2.5 rounded-lg text-[12px]
                bg-red-500/10 text-red-500" transition:fade={{ duration: 120 }}>
        <Icon icon="lucide:alert-triangle" width="13" class="shrink-0 mt-0.5" />
        <span class="min-w-0">{error}</span>
      </p>
    {:else if loading && !report}
      <!-- Le squelette reprend la forme des lignes : la mise en page ne
           bouge pas quand les constats arrivent. -->
      <div class="space-y-2" aria-hidden="true">
        {#each Array.from({ length: 5 }) as _, i (i)}
          <div class="h-14 rounded-xl animate-pulse bg-neutral-100/70 dark:bg-white/4"></div>
        {/each}
      </div>
    {:else if report && sections.length === 0}
      <div class="flex flex-col items-center justify-center gap-3 py-20 text-center">
        <Icon icon="lucide:check-circle-2" width="34" class="text-emerald-500" />
        <p class="text-[14px] font-medium text-neutral-700 dark:text-neutral-200">
          {$t("audit.all_good")}
        </p>
        <p class="max-w-sm text-[12px] leading-relaxed
                  text-neutral-400 dark:text-neutral-500">
          {$t("audit.all_good_hint")}
        </p>
      </div>
    {:else}
      <div class="space-y-6">
        {#each sections as section (section.severity)}
          <div>
            <div class="flex items-center gap-2 mb-2 px-1">
              <Icon icon={ICON[section.severity]} width="13"
                    class="shrink-0 {ACCENT[section.severity]}" />
              <span class="text-[10px] font-semibold uppercase tracking-widest
                           {ACCENT[section.severity]}">
                {$t(`audit.severity.${section.severity}`)}
              </span>
              <span class="flex-1 h-px bg-linear-to-r
                           from-neutral-200 to-transparent
                           dark:from-white/10 dark:to-transparent"></span>
            </div>

            <div class="space-y-1.5">
              {#each section.groups as group (group.kind)}
                {@const label = $t(`audit.kind.${group.kind}`)}
                <button
                  type="button"
                  onclick={() => onpick(group, label)}
                  class="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-left
                         cursor-pointer transition-colors
                         bg-neutral-100/70 dark:bg-white/3
                         hover:bg-neutral-200/70 dark:hover:bg-white/7"
                >
                  <span class="min-w-0 flex-1">
                    <span class="block text-[13px] font-medium
                                 text-neutral-800 dark:text-neutral-100">
                      {label}
                    </span>
                    <!-- Les exemples valent mieux qu'une description : ils
                         disent si le constat concerne ce qu'on croit. -->
                    <span class="block text-[11px] truncate
                                 text-neutral-400 dark:text-neutral-500">
                      {group.samples.join(" · ")}
                      {#if group.count > group.samples.length}
                        …
                      {/if}
                    </span>
                  </span>

                  <span class="shrink-0 px-2 py-0.5 rounded-md text-[11px] font-medium tabular-nums
                               bg-neutral-200/80 dark:bg-white/10
                               text-neutral-600 dark:text-neutral-300">
                    {group.count}
                  </span>

                  <Icon icon="lucide:arrow-right" width="14"
                        class="shrink-0 text-neutral-300 dark:text-neutral-600" />
                </button>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
