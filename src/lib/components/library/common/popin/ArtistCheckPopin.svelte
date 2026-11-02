<script lang="ts">
  // Artistes multiples : la vérification en direct, puis le bilan (séparations annulables, noms à trancher).
  // Même en-tête (tuiles de chiffres) pendant et après, pour que rien ne saute.
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fade, fly } from "svelte/transition";
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { t, currentLocale } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
  import { artistCheck, type CheckReport, type RuleView } from "#lib/stores/library/artistCheck.store";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import ArtistAvatar from "#lib/components/ui/image/ArtistAvatar.svelte";

  let { one = null }: { one?: string | null } = $props();

  const etat = $derived($artistCheck);
  let report = $state<CheckReport | null>(null);
  let erreur = $state<string | null>(null);
  let working = $state<string | null>(null);
  let annules = $state(new Set<string>());
  let tranches = $state(new Map<string, "split" | "whole">());
  let touche = false;
  let corps = $state<HTMLElement | null>(null);

  // Bilan : dès que la vérification est finie (ou tout de suite, pour revoir le dernier).
  let charge = -1;
  $effect(() => {
    if (etat.running) return;
    const run = etat.runId;
    if (charge === (run ?? 0)) return;
    charge = run ?? 0;
    invoke<CheckReport>("artist_check_report", { runId: run })
      .then((r) => (report = r))
      .catch((e) => (erreur = String(e)));
  });

  const sources: Record<string, string> = { tag: "artist_split.source_tag", user: "artist_split.source_user" };
  const source = (s: string) => (sources[s] ? $t(sources[s]) : "MusicBrainz");
  const photo = (nom: string) => {
    const k = nom.trim().toLowerCase();
    return report?.images[k] ?? etat.images[k] ?? null;
  };
  // Coupé net à 30 caractères : la limite des vieux tags ID3v1, la fin manque sans doute.
  const coupe = (nom: string) => [...nom].length === 30;

  async function regle(nom: string, decision: "split" | "whole", parts: string[] = []) {
    working = nom;
    erreur = null;
    try {
      await invoke("artist_rule_set", { name: nom, decision, parts });
      touche = true;
      return true;
    } catch (e) {
      erreur = String(e);
      return false;
    } finally {
      working = null;
    }
  }
  async function annuler(v: RuleView) {
    if (await regle(v.name, "whole")) annules = new Set(annules).add(v.name);
  }
  async function retablir(v: RuleView) {
    if (await regle(v.name, "split", v.parts)) {
      const s = new Set(annules);
      s.delete(v.name);
      annules = s;
    }
  }
  async function toutAnnuler() {
    for (const v of report?.split ?? []) if (!annules.has(v.name)) await annuler(v);
  }
  // Possible dès le direct : la vérification ne repasse jamais sur un nom déjà vu.
  async function trancher(v: RuleView, d: "split" | "whole") {
    if (await regle(v.name, d, d === "split" ? v.parts : [])) tranches = new Map(tranches).set(v.name, d);
  }

  function fermer() {
    popinStore.close();
    if (touche) libraryContentStore.refresh();
  }
  const aller = (id: string) => corps?.querySelector(`#${id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });

  const fmt = (n: number) => Math.round(n).toLocaleString($currentLocale);
  const compte = (d: string) => etat.live.filter((x) => x.decision === d).length;
  const restants = $derived((report?.split ?? []).filter((v) => !annules.has(v.name)).length);
  const aTrancher = $derived((report?.unknown ?? []).filter((v) => !tranches.has(v.name)).length);

  // Chiffres des tuiles : en direct pendant la vérification, ceux du bilan ensuite ; ils glissent d'une valeur à l'autre.
  const chiffres = $derived(
    etat.running
      ? { split: compte("split"), whole: compte("whole"), unknown: compte("unknown") }
      : { split: report?.split.length ?? 0, whole: report?.whole.length ?? 0, unknown: report?.run_unknown ?? 0 },
  );
  const tw = { duration: 450, easing: cubicOut };
  const nSplit = Tween.of(() => chiffres.split, tw);
  const nWhole = Tween.of(() => chiffres.whole, tw);
  const nUnknown = Tween.of(() => chiffres.unknown, tw);
  const total = $derived(chiffres.split + chiffres.whole + chiffres.unknown);
  const pct = $derived(etat.total ? Math.min(100, (etat.done / etat.total) * 100) : 0);
  const nPct = Tween.of(() => pct, { duration: 600, easing: cubicOut });

  // Temps restant, d'après le rythme jusqu'ici (MusicBrainz : environ un nom par seconde).
  const reste = $derived.by(() => {
    if (!etat.running || etat.done < 3 || etat.total <= etat.done) return "";
    const s = ((Date.now() - etat.startedAt) / etat.done) * (etat.total - etat.done) / 1000;
    return s < 60 ? $t("artist_split.eta_less") : $t("artist_split.eta_min").replace("{n}", String(Math.ceil(s / 60)));
  });

  // Pendant la vérification : les plus récents en haut, sans les noms confirmés (rien n'a changé pour eux).
  const fil = $derived([...etat.live].reverse().filter((x) => x.decision !== "whole").slice(0, 40));
  const entree = (i: number) => ({ y: 8, duration: 280, delay: Math.min(i, 14) * 28, easing: cubicOut });

  const seg = "flex items-center gap-1.5 h-7.5 px-2.5 rounded-[9px] text-[12px] font-semibold cursor-pointer transition-[background-color,color,box-shadow] duration-150 disabled:opacity-40 disabled:cursor-default";
  const lien = "shrink-0 flex items-center gap-1.5 h-8 px-3 rounded-xl text-[12px] font-semibold cursor-pointer transition-colors text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx) disabled:opacity-40 disabled:cursor-default";
  const ligne = "group flex flex-wrap items-center gap-x-3.5 gap-y-2.5 p-3 rounded-2xl border transition-[background-color,border-color,box-shadow] duration-300";
  const survol = "hover:border-(--rg-bd2) hover:shadow-[0_6px_18px_-12px_rgb(0_0_0/0.5)]";
</script>

<!-- Les artistes d'un nom, en pile de pastilles (3 places au plus) ; largeur fixe pour aligner les noms. -->
{#snippet pile(noms: string[], terne = false, coche = false)}
  <span class="w-19 shrink-0 flex items-center">
  <span class="relative inline-flex items-center {terne ? 'opacity-45 grayscale' : ''} transition-[opacity,filter] duration-300">
    {#each noms.slice(0, noms.length > 3 ? 2 : 3) as n, i (i)}
      <span class="rounded-full ring-[2.5px] ring-(--rg-creux) {i ? '-ml-2.5' : ''}">
        <ArtistAvatar name={n} path={photo(n)} class="w-8 h-8 text-[12px]" />
      </span>
    {/each}
    {#if noms.length > 3}
      <span class="-ml-2.5 w-8 h-8 rounded-full ring-[2.5px] ring-(--rg-creux) bg-(--rg-s2) text-(--rg-tx2) flex items-center justify-center text-[11px] font-bold">+{noms.length - 2}</span>
    {/if}
    {#if coche}
      <span class="absolute -right-1 -bottom-1 w-4.5 h-4.5 rounded-full bg-(--rg-g) text-white ring-2 ring-(--rg-gbg) flex items-center justify-center" in:fly={{ y: 4, duration: 250 }}>
        <Icon icon="material-symbols:check-rounded" width="12" />
      </span>
    {/if}
  </span>
  </span>
{/snippet}

<!-- Les noms proposés, sobres : la couleur est réservée à l'état. -->
{#snippet noms(parts: string[], coupe = false)}
  <span class="flex flex-wrap items-center gap-x-1 gap-y-1 min-w-0 text-[12px]">
    {#each parts as p, i (i)}
      {#if i}<span class="text-(--rg-mu2)" aria-hidden="true">+</span>{/if}
      <span class="px-2 py-0.5 rounded-md border border-(--rg-bd) bg-(--rg-champ) font-medium text-(--rg-tx2)">{p}</span>
    {/each}
    {#if coupe}
      <span class="ml-1 flex items-center gap-1 px-1.5 py-0.5 rounded-md text-[10.5px] font-semibold border border-(--rg-ambd) text-(--rg-am)" title={$t("artist_split.truncated_hint")}>
        <Icon icon="material-symbols:content-cut-rounded" width="11" />{$t("artist_split.truncated")}
      </span>
    {/if}
  </span>
{/snippet}

<!-- Une tuile de chiffre ; cliquable quand sa section est là. -->
{#snippet tuile(valeur: number, libelle: string, teinte: string, icone: string, cible: string | null)}
  {@const actif = !!cible && !etat.running}
  <button type="button" disabled={!actif} onclick={() => cible && aller(cible)}
          class="group/t flex items-center gap-2.5 min-w-0 px-3 py-2.5 rounded-xl border border-(--rg-bd) bg-(--rg-creux) text-left transition-[border-color,background-color] duration-150
                 {actif ? 'cursor-pointer hover:border-(--rg-bd2) hover:bg-(--rg-s2)/60' : 'cursor-default'}">
    <span class="w-7 h-7 shrink-0 rounded-lg flex items-center justify-center {teinte}"><Icon icon={icone} width="16" /></span>
    <span class="flex-1 min-w-0">
      <span class="block text-[19px] leading-none font-bold tabular-nums text-(--rg-tx)">{fmt(valeur)}</span>
      <span class="block mt-1 truncate text-[11.5px] text-(--rg-mu)">{libelle}</span>
    </span>
    {#if actif}
      <Icon icon="material-symbols:arrow-downward-rounded" width="15" class="shrink-0 text-(--rg-mu2) transition-transform group-hover/t:translate-y-0.5 group-hover/t:text-(--rg-tx2)" />
    {/if}
  </button>
{/snippet}

<!-- Un nom à trancher : la commande segmentée, puis la décision prise (modifiable). -->
{#snippet aTrancherLigne(v: RuleView, i: number)}
  {@const fait = tranches.get(v.name)}
  {@const parts = v.parts.length >= 2 ? v.parts : [v.name]}
  <div class="{ligne} {fait ? 'border-(--rg-gbd) bg-(--rg-gbg) artiste-tranche' : `border-(--rg-ambd) bg-(--rg-creux) ${survol}`}" in:fly={entree(i)}>
    {@render pile(fait === "whole" ? [v.name] : parts, false, !!fait)}
    <div class="min-w-0 flex-1 basis-56">
      <p class="text-[13px] font-semibold truncate text-(--rg-tx)" title={v.name}>{v.name}</p>
      <div class="mt-1.5">
        {#if fait}
          <p class="text-[12px] font-medium text-(--rg-gtx)" in:fade={{ duration: 200 }}>
            {fait === "split" ? $t("artist_split.split_into").replace("{n}", String(v.parts.length)) : $t("artist_split.done_whole")}
          </p>
        {:else if v.parts.length >= 2}
          {@render noms(v.parts, coupe(v.name))}
        {/if}
      </div>
    </div>
    {#if fait}
      {#if v.parts.length >= 2}
        <button type="button" class={lien} disabled={working !== null} onclick={() => trancher(v, fait === "split" ? "whole" : "split")}>
          <Icon icon="material-symbols:swap-horiz-rounded" width="15" />{$t("artist_split.change")}
        </button>
      {/if}
    {:else}
      <span class="ml-auto shrink-0 inline-flex p-0.5 rounded-xl border border-(--rg-bd) bg-(--rg-champ)" role="group" aria-label={v.name}>
        {#if v.parts.length >= 2}
          <button type="button" class="{seg} text-(--rg-gtx) hover:bg-(--rg-gbg) hover:shadow-[inset_0_0_0_1px_var(--rg-gbd)]" disabled={working !== null} onclick={() => trancher(v, "split")}>
            <Icon icon="material-symbols:call-split-rounded" width="15" />{$t("artist_split.split")}
          </button>
          <span class="w-px my-1.5 bg-(--rg-bd)" aria-hidden="true"></span>
        {/if}
        <button type="button" class="{seg} text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)" disabled={working !== null} onclick={() => trancher(v, "whole")}>
          <Icon icon="material-symbols:person-outline-rounded" width="15" />{$t("artist_split.keep")}
        </button>
      </span>
    {/if}
  </div>
{/snippet}

<div class="flex-1 min-h-0 flex flex-col">
  <!-- ─── En-tête : où on en est ─── -->
  <div class="shrink-0 px-6 pt-5 pb-4 border-b border-(--rg-bd) bg-linear-180 from-(--rg-g)/6 to-transparent">
    <div class="flex items-center gap-3.5">
      {#if etat.running}
        <!-- Anneau qui tourne : ça cherche. -->
        <span class="relative w-11 h-11 shrink-0 rounded-full flex items-center justify-center bg-(--rg-gbg) text-(--rg-g)">
          <svg class="absolute inset-0 w-full h-full animate-spin [animation-duration:1.3s]" viewBox="0 0 44 44" aria-hidden="true">
            <circle cx="22" cy="22" r="20.5" fill="none" stroke="currentColor" stroke-opacity="0.18" stroke-width="2.5" />
            <circle cx="22" cy="22" r="20.5" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-dasharray="34 95" />
          </svg>
          <Icon icon="material-symbols:travel-explore-rounded" width="20" />
        </span>
      {:else}
        <span class="w-11 h-11 shrink-0 rounded-full border flex items-center justify-center bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-g)" in:fade={{ duration: 200 }}>
          <Icon icon="material-symbols:check-circle-outline-rounded" width="22" />
        </span>
      {/if}
      <div class="flex-1 min-w-0">
        <p class="truncate text-[15px] font-bold text-(--rg-tx)">
          {#if etat.running}{$t(one ? "artist_split.live_one" : "artist_split.live_title")}
          {:else if report?.run}{$t("artist_split.summary").replace("{n}", fmt(report.run.checked))}
          {:else}{$t("artist_split.title")}{/if}
        </p>
        <p class="mt-0.5 truncate text-[12.5px] text-(--rg-mu)">
          {#if etat.running}
            {#if etat.total > 1}
              <span class="tabular-nums">{$t("artist_split.names_done").replace("{done}", fmt(etat.done)).replace("{total}", fmt(etat.total))}</span>{#if reste}<span class="text-(--rg-mu2)">{"  ·  "}</span>{reste}{/if}
            {:else}{one || $t("artist_split.live_preparing")}{/if}
          {:else if report?.run?.error || etat.error}<span class="text-(--rg-am)">{$t("artist_split.interrupted")}</span>
          {:else}{$t("artist_split.footnote_short")}{/if}
        </p>
      </div>
      {#if etat.running && etat.total > 1}
        <span class="shrink-0 text-[28px] leading-none font-bold tabular-nums tracking-tight text-(--rg-g)">{Math.floor(nPct.current)}<span class="ml-0.5 text-[15px] font-semibold text-(--rg-gtx)">%</span></span>
      {/if}
    </div>

    <!-- Progression (rayures qui défilent) pendant la vérification ; répartition des résultats ensuite. -->
    <div class="relative mt-4 h-2 rounded-full overflow-hidden bg-(--rg-s2)">
      {#if etat.running}
        {#if etat.total > 1}
          <div class="h-full rounded-full bg-(--rg-g) artiste-rayures transition-[width] duration-500 ease-out" style="width: {Math.max(2, pct)}%"></div>
        {:else}
          <div class="h-full w-1/3 rounded-full bg-(--rg-g)/75 animate-[artiste-balayage_1.4s_ease-in-out_infinite]"></div>
        {/if}
      {:else if total}
        <div class="flex h-full gap-0.5" in:fade={{ duration: 300 }}>
          {#if chiffres.split}<span class="h-full rounded-full bg-(--rg-g) transition-[flex-grow] duration-500" style="flex-grow: {chiffres.split}"></span>{/if}
          {#if chiffres.whole}<span class="h-full rounded-full bg-(--rg-mu2) transition-[flex-grow] duration-500" style="flex-grow: {chiffres.whole}"></span>{/if}
          {#if chiffres.unknown}<span class="h-full rounded-full bg-(--rg-am) transition-[flex-grow] duration-500" style="flex-grow: {chiffres.unknown}"></span>{/if}
        </div>
      {/if}
    </div>

    <div class="mt-3.5 grid grid-cols-3 gap-2">
      {@render tuile(nSplit.current, $t("artist_split.section_split"), "bg-(--rg-gbg) text-(--rg-g)", "material-symbols:call-split-rounded", report?.split.length ? "artistes-separes" : null)}
      {@render tuile(nWhole.current, $t("artist_split.stat_whole"), "bg-(--rg-s2) text-(--rg-tx2)", "material-symbols:person-outline-rounded", null)}
      {@render tuile(nUnknown.current, $t("artist_split.section_unknown"), "bg-(--rg-ambg) text-(--rg-am)", "material-symbols:help-outline-rounded", report?.unknown.length ? "artistes-trancher" : null)}
    </div>
  </div>

  <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app px-6 py-5" bind:this={corps}>
    {#if erreur}
      <p class="flex items-start gap-2 px-4 py-3 mb-3 rounded-xl border border-red-500/25 bg-red-500/8 text-sm text-red-600 dark:text-red-400">
        <Icon icon="material-symbols:warning-outline-rounded" width="17" class="shrink-0 mt-px" /><span class="min-w-0">{erreur}</span>
      </p>
    {/if}

    {#if etat.running}
      <div class="space-y-2">
        <!-- Le nom en cours de recherche, toujours en haut. -->
        <div class="relative overflow-hidden flex items-center gap-3.5 p-3 rounded-2xl border border-dashed border-(--rg-gbd) bg-(--rg-creux)">
          <span class="absolute inset-0 artiste-balaie pointer-events-none" aria-hidden="true"></span>
          <span class="w-19 shrink-0 flex items-center">
            <span class="relative w-8 h-8 rounded-full flex items-center justify-center text-(--rg-g)">
              <svg class="absolute inset-0 w-full h-full animate-spin [animation-duration:0.9s]" viewBox="0 0 32 32" aria-hidden="true">
                <circle cx="16" cy="16" r="14" fill="none" stroke="currentColor" stroke-opacity="0.2" stroke-width="2.5" />
                <circle cx="16" cy="16" r="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-dasharray="22 66" />
              </svg>
              <Icon icon="material-symbols:search-rounded" width="15" />
            </span>
          </span>
          <div class="relative min-w-0 flex-1">
            {#key etat.current}
              <p class="text-[13px] font-semibold truncate text-(--rg-tx)" in:fly={{ y: 6, duration: 220 }}>{etat.current || one || $t("artist_split.live_preparing")}</p>
            {/key}
            <p class="mt-0.5 text-[12px] text-(--rg-gtx)">{$t("artist_split.live_searching")}</p>
          </div>
        </div>

        {#each fil as x, i (x.name)}
          {#if x.decision === "split"}
            <div class="{ligne} border-(--rg-bd) bg-(--rg-creux)" in:fly={{ y: -10, duration: 300, easing: cubicOut }}>
              {@render pile(x.parts)}
              <div class="min-w-0 flex-1 basis-56">
                <p class="text-[13px] font-semibold truncate text-(--rg-tx)" title={x.name}>{x.name}</p>
                <div class="mt-1.5">{@render noms(x.parts)}</div>
              </div>
              <!-- Un état, pas un bouton : du texte, sans fond. -->
              <span class="shrink-0 flex items-center gap-1 text-[12px] font-semibold text-(--rg-gtx)">
                <Icon icon="material-symbols:check-rounded" width="15" />{$t("artist_split.done_split")}
              </span>
            </div>
          {:else}
            {@render aTrancherLigne({ name: x.name, parts: x.parts, source: "musicbrainz" }, i)}
          {/if}
        {/each}

        {#if !fil.length}
          <p class="pt-6 text-center text-[12.5px] text-(--rg-mu) text-pretty">{$t("artist_split.live_hint")}</p>
        {/if}
      </div>

    {:else if report}
      <!-- À trancher en premier : c'est là qu'on attend l'utilisateur. -->
      {#if report.unknown.length}
        <div id="artistes-trancher" class="scroll-mt-2 flex items-center gap-2 mb-1">
          <h3 class="text-[12px] font-bold uppercase tracking-[0.09em] text-(--rg-tx2)">{$t("artist_split.section_unknown")}</h3>
          <span class="px-1.75 py-0.5 rounded-md font-mono text-[11px] font-bold bg-(--rg-ambg) text-(--rg-am)">{fmt(aTrancher)}</span>
        </div>
        <p class="mb-3 text-[12.5px] text-(--rg-mu) text-pretty">{$t("artist_split.section_unknown_desc")}</p>
        <div class="space-y-2 mb-7">
          {#each report.unknown as v, i (v.name)}{@render aTrancherLigne(v, i)}{/each}
        </div>
      {/if}

      <!-- Séparés : annulables un par un ou tous -->
      {#if report.split.length}
        <div id="artistes-separes" class="scroll-mt-2 flex items-center gap-2 mb-3">
          <h3 class="text-[12px] font-bold uppercase tracking-[0.09em] text-(--rg-tx2)">{$t("artist_split.section_split")}</h3>
          <span class="px-1.75 py-0.5 rounded-md font-mono text-[11px] font-bold bg-(--rg-gbg) text-(--rg-gtx)">{fmt(restants)}</span>
          {#if restants > 1}
            <button type="button" class="{lien} ml-auto" disabled={working !== null} onclick={toutAnnuler}>
              <Icon icon="lucide:undo-2" width="14" />{$t("artist_split.undo_all")}
            </button>
          {/if}
        </div>
        <div class="space-y-2 mb-6">
          {#each report.split as v, i (v.name)}
            {@const annule = annules.has(v.name)}
            <div class="{ligne} border-(--rg-bd) bg-(--rg-creux) {survol}" in:fly={entree(i + report.unknown.length)}>
              {@render pile(annule ? [v.name] : v.parts, annule)}
              <div class="min-w-0 flex-1 basis-56">
                <p class="flex items-center gap-2 min-w-0">
                  <span class="text-[13px] font-semibold truncate {annule ? 'text-(--rg-mu)' : 'text-(--rg-tx)'}" title={v.name}>{v.name}</span>
                  <span class="shrink-0 flex items-center gap-1 text-[11px] text-(--rg-mu)">
                    <Icon icon={v.source === "tag" ? "material-symbols:sell-outline-rounded" : v.source === "user" ? "material-symbols:person-check-outline-rounded" : "material-symbols:public"} width="12" />
                    {source(v.source)}
                  </span>
                </p>
                <div class="mt-1.5">
                  {#if annule}
                    <p class="text-[12px] text-(--rg-mu)" in:fade={{ duration: 200 }}>{$t("artist_split.done_whole")}</p>
                  {:else}
                    {@render noms(v.parts)}
                  {/if}
                </div>
              </div>
              {#if annule}
                <button type="button" class={lien} disabled={working !== null} onclick={() => retablir(v)}>
                  <Icon icon="material-symbols:call-split-rounded" width="15" />{$t("artist_split.redo")}
                </button>
              {:else}
                <button type="button" class="{lien} opacity-70 group-hover:opacity-100 focus-visible:opacity-100" disabled={working !== null} onclick={() => annuler(v)}>
                  <Icon icon="lucide:undo-2" width="14" />{$t("artist_split.undo")}
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if !report.split.length && !report.unknown.length}
        <div class="flex flex-col items-center justify-center gap-3 py-12 text-center text-[13px] text-(--rg-mu)">
          <span class="w-12 h-12 rounded-2xl flex items-center justify-center bg-(--rg-gbg) border border-(--rg-gbd) text-(--rg-g)">
            <Icon icon="material-symbols:done-all-rounded" width="24" />
          </span>
          {$t("artist_split.nothing")}
        </div>
      {/if}

      <p class="flex gap-2 px-1 text-xs text-(--rg-mu) text-pretty">
        <Icon icon="material-symbols:info-outline-rounded" width="15" class="shrink-0 mt-px text-(--rg-mu2)" />
        {$t("artist_split.footnote")}
      </p>
    {:else}
      <div class="space-y-2">
        {#each [0, 1, 2] as i (i)}<div class="h-16 rounded-2xl animate-pulse bg-(--rg-creux)"></div>{/each}
      </div>
    {/if}
  </div>

  {#if etat.running}
    <!-- Pendant la vérification : on peut fermer, elle continue. -->
    <footer class="shrink-0 flex items-center gap-3 px-6 py-3.5 border-t border-(--rg-bd) bg-(--rg-creux)">
      <span class="relative flex w-2 h-2 shrink-0" aria-hidden="true">
        <span class="absolute inset-0 rounded-full bg-(--rg-g) animate-ping"></span>
        <span class="relative w-2 h-2 rounded-full bg-(--rg-g)"></span>
      </span>
      <span class="flex-1 min-w-0 truncate text-[12.5px] text-(--rg-mu)">{$t("artist_split.background_hint")}</span>
      <button type="button" onclick={fermer}
              class="shrink-0 flex items-center gap-2 h-10 pl-3.5 pr-4 rounded-xl border border-(--rg-bd) bg-(--rg-champ) text-sm font-semibold text-(--rg-tx2) cursor-pointer
                     transition-[border-color,color,background-color] duration-150 hover:border-(--rg-gbd) hover:bg-(--rg-gbg) hover:text-(--rg-gtx)">
        <Icon icon="material-symbols:close-fullscreen-rounded" width="16" />{$t("artist_split.hide")}
      </button>
    </footer>
  {:else}
    <PopinFooter cancelLabel={$t("batch.close")} oncancel={fermer} />
  {/if}
</div>

<style>
  @keyframes -global-artiste-balayage {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(300%); }
  }
  /* Barre de progression : rayures qui avancent. */
  :global(.artiste-rayures) {
    background-image: repeating-linear-gradient(-45deg, rgb(255 255 255 / 0.22) 0 6px, transparent 6px 12px);
    background-size: 17px 17px;
    animation: artiste-defile 0.7s linear infinite;
  }
  @keyframes -global-artiste-defile {
    to { background-position: 17px 0; }
  }
  /* Le nom en cours : un reflet qui passe. */
  :global(.artiste-balaie) {
    background: linear-gradient(100deg, transparent 30%, color-mix(in oklab, var(--rg-g) 10%, transparent) 50%, transparent 70%);
    background-size: 250% 100%;
    animation: artiste-reflet 1.8s ease-in-out infinite;
  }
  @keyframes -global-artiste-reflet {
    0% { background-position: 120% 0; }
    100% { background-position: -20% 0; }
  }
  /* Décision prise : un bref éclat vert. */
  :global(.artiste-tranche) {
    animation: artiste-eclat 0.6s ease-out;
  }
  @keyframes -global-artiste-eclat {
    0% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--rg-g) 45%, transparent); }
    100% { box-shadow: 0 0 0 10px transparent; }
  }
</style>
