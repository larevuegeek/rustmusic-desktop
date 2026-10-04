<script lang="ts">
  // Comprendre le son : schéma interactif du trajet, choix du mode, pastilles du lecteur, glossaire.
  import Icon from "@iconify/svelte";
  import { tick } from "svelte";
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import OptionGroup from "$lib/components/ui/input/OptionGroup.svelte";
  import OptionBlock from "$lib/components/ui/input/OptionBlock.svelte";
  import SegmentedControl from "$lib/components/ui/input/SegmentedControl.svelte";
  import TexteAide from "$lib/components/ui/aide/TexteAide.svelte";
  import { TERMES } from "$lib/config/glossaire";
  import { CHAINE } from "$lib/helper/audio/chaineAudio";
  import { texteBrut } from "$lib/helper/tools/texteAide";
  import type { PipelineMode } from "$lib/stores/player/playbackPipeline.store";

  // `h` : teinte de la tuile (oklch).
  const ETAPES = [
    { cle: "file", icone: "material-symbols:audio-file-outline-rounded", h: 250 },
    { cle: "decode", icone: "material-symbols:unarchive-outline-rounded", h: 155 },
    { cle: "dsp", icone: "material-symbols:graphic-eq-rounded", h: 70 },
    { cle: "mixer", icone: "material-symbols:tune-rounded", h: 300 },
    { cle: "dac", icone: "material-symbols:headphones-outline-rounded", h: 200 },
  ];
  const MODES = [
    { id: "shared", icone: "material-symbols:group-outline-rounded", h: 230, plus: ["pro1", "pro2"], moins: ["con1"] },
    { id: "exclusive", icone: "material-symbols:lock-outline-rounded", h: 150, plus: ["pro1", "pro2"], moins: ["con1", "con2"] },
  ] as const;
  const RESULTAT: Record<string, PipelineMode[]> = { shared: ["shared", "resampled"], exclusive: ["bit-perfect", "dop"] };
  const PASTILLES = ["bit-perfect", "exclusive", "dop", "resampled", "dsd", "shared"] as const;

  let mode = $state<"shared" | "exclusive">("shared");
  let etape = $state(1);
  const choisie = $derived(ETAPES[etape]);
  const contourne = (cle: string) => mode === "exclusive" && cle === "mixer";
  const descEtape = $derived(
    contourne(choisie.cle) ? $t("guide.step_mixer_desc_exclusive") : $t(`guide.step_${choisie.cle}_desc`),
  );

  // Arrivée par « En savoir plus » ou par l'index : le terme défile au centre et s'éclaire.
  let cible = $state<string | null>(null);
  let extinction: ReturnType<typeof setTimeout> | undefined;
  async function aller(id: string) {
    await tick();
    document.getElementById(id)?.scrollIntoView({ block: "center", behavior: "smooth" });
    cible = id;
    clearTimeout(extinction);
    extinction = setTimeout(() => (cible = null), 2000);
  }
  $effect(() => {
    const id = page.url.hash.slice(1);
    if (id) aller(id);
  });
  $effect(() => {
    const surTerme = (e: Event) => aller((e as CustomEvent<string>).detail);
    window.addEventListener("aide-terme", surTerme);
    return () => {
      window.removeEventListener("aide-terme", surTerme);
      clearTimeout(extinction);
    };
  });

  const pastille = "h-5.5 px-2 inline-flex items-center gap-1.5 rounded-full whitespace-nowrap text-[10.5px] font-semibold border";
</script>

<!-- Les teintes du lecteur, hors lecteur : le neutre prend les tons des réglages. -->
<div class="contents" style="--lc-survol: var(--rg-s2); --lc-trait: var(--rg-bd2); --lc-tx2: var(--rg-tx2)">

<!-- ─── Le trajet du son ─── -->
<OptionGroup title={$t("guide.path_title")} hint={$t("guide.path_hint")}>
  <OptionBlock keywords="{$t('guide.path_title')} {ETAPES.map((e) => $t(`guide.step_${e.cle}_title`)).join(' ')}">
    <div class="flex items-start justify-between gap-5 max-md:flex-col px-6 pt-5">
      <p class="max-w-[52ch] text-sm leading-normal text-(--rg-tx2) text-pretty">{$t("guide.path_intro")}</p>
      <SegmentedControl
        value={mode}
        options={[
          { value: "shared", label: $t("glossary.shared.name") },
          { value: "exclusive", label: $t("glossary.exclusive.name") },
        ]}
        label={$t("guide.modes_title")}
        onchange={(v) => (mode = v as "shared" | "exclusive")}
      />
    </div>

    <!-- Le schéma : cinq étapes reliées ; en exclusif, le mixer s'efface. -->
    <div class="relative grid grid-cols-5 px-6 pt-7">
      <span
        class="absolute left-[calc(10%+12px)] right-[calc(10%+12px)] top-[55px] h-0.5 rounded-full transition-colors duration-300
               {mode === 'exclusive' ? 'bg-(--rg-g)' : 'bg-(--rg-bd2)'}"
        aria-hidden="true"
      ></span>
      {#each ETAPES as e, i (e.cle)}
        {@const efface = contourne(e.cle)}
        <button
          type="button"
          class="relative flex flex-col items-center gap-2 px-1 text-center cursor-pointer group"
          aria-pressed={etape === i}
          onclick={() => (etape = i)}
        >
          <span
            class="rg-icone-teinte w-14 h-14 flex items-center justify-center rounded-2xl transition-all duration-300
                   {etape === i ? 'ring-2 ring-(--rg-g) ring-offset-[3px] ring-offset-(--rg-carte)' : 'group-hover:scale-105'}
                   {efface ? 'opacity-30 grayscale' : ''}"
            style="--h: {e.h}"
          >
            <Icon icon={e.icone} width="26" />
          </span>
          <span class="text-[13px] font-semibold leading-tight {efface ? 'text-(--rg-mu2) line-through' : 'text-(--rg-tx)'}">
            {$t(`guide.step_${e.cle}_title`)}
          </span>
          <span class="text-[11.5px] leading-tight {efface ? 'font-semibold text-(--rg-gtx)' : 'text-(--rg-mu)'}">
            {efface ? $t("guide.step_mixer_bypassed") : $t(`guide.step_${e.cle}_short`)}
          </span>
        </button>
      {/each}
    </div>

    <!-- Le détail de l'étape choisie, pointé par un bec. -->
    <div class="relative mx-6 mt-4 mb-5 flex gap-3.5 px-5 py-4 rounded-xl border border-(--rg-bd) bg-(--rg-creux)">
      <span
        class="absolute -top-[7px] w-3 h-3 rotate-45 border-l border-t border-(--rg-bd) bg-(--rg-creux) transition-[left] duration-300"
        style="left: calc({(etape + 0.5) * 20}% - 6px)"
        aria-hidden="true"
      ></span>
      <span class="rg-icone-teinte shrink-0 w-9 h-9 flex items-center justify-center rounded-xl" style="--h: {choisie.h}">
        <Icon icon={choisie.icone} width="19" />
      </span>
      <div class="min-w-0">
        <p class="text-[15px] font-bold leading-tight text-(--rg-tx)">
          <span class="font-mono text-xs text-(--rg-mu) mr-1.5">{etape + 1}/5</span>{$t(`guide.step_${choisie.cle}_title`)}
        </p>
        <p class="mt-1.5 text-sm leading-[1.55] text-(--rg-tx2) text-pretty"><TexteAide texte={descEtape} /></p>
      </div>
    </div>

    <div class="flex items-center flex-wrap gap-x-3 gap-y-2 px-6 py-3.5 border-t border-(--rg-line) bg-(--rg-s2)/50">
      <span class="text-[11px] font-bold uppercase tracking-[0.08em] text-(--rg-mu)">{$t("guide.result")}</span>
      <span class="text-[13px] text-(--rg-tx2)">{$t(`guide.result_${mode}`)}</span>
      <span class="flex gap-1.5 ml-auto">
        {#each RESULTAT[mode] as p (p)}
          <span class="{pastille} {CHAINE[p].teinte}"><span class="w-1.5 h-1.5 rounded-full bg-current"></span>{$t(CHAINE[p].cle)}</span>
        {/each}
      </span>
    </div>
  </OptionBlock>
</OptionGroup>

<!-- ─── Partagé ou exclusif ─── -->
<OptionGroup title={$t("guide.modes_title")} hint={$t("guide.modes_hint")}>
  <OptionBlock keywords="{$t('guide.modes_title')} {$t('glossary.shared.name')} {$t('glossary.exclusive.name')}" class="flex flex-col gap-4 p-4">
    <div class="grid grid-cols-2 max-md:grid-cols-1 gap-3">
      {#each MODES as m (m.id)}
        <div class="min-w-0 flex flex-col px-5 pt-4.5 pb-5 rounded-xl border border-(--rg-bd) bg-(--rg-creux)">
          <div class="flex items-center gap-3">
            <span class="rg-icone-teinte shrink-0 w-10 h-10 flex items-center justify-center rounded-xl" style="--h: {m.h}">
              <Icon icon={m.icone} width="21" />
            </span>
            <p class="text-base font-bold leading-tight text-(--rg-tx)">{$t(`glossary.${m.id}.name`)}</p>
          </div>
          <p class="mt-3 text-[13.5px] leading-normal text-(--rg-tx2) text-pretty">{$t(`guide.mode_${m.id}_for`)}</p>
          <ul class="mt-3.5 flex flex-col gap-2 text-[13px] leading-snug">
            {#each m.plus as c (c)}
              <li class="flex gap-2 text-(--rg-tx)">
                <Icon icon="material-symbols:check-circle-rounded" width="17" class="shrink-0 text-(--rg-g)" />{$t(`guide.mode_${m.id}_${c}`)}
              </li>
            {/each}
            {#each m.moins as c (c)}
              <li class="flex gap-2 text-(--rg-mu)">
                <Icon icon="material-symbols:do-not-disturb-on-outline-rounded" width="17" class="shrink-0 text-(--rg-am)" />{$t(`guide.mode_${m.id}_${c}`)}
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
    <a href="/settings/audio" class="self-start flex items-center gap-1 px-1 text-[13px] font-semibold text-(--rg-gtx) hover:underline">
      {$t("guide.modes_action")}<span aria-hidden="true">→</span>
    </a>
  </OptionBlock>
</OptionGroup>

<!-- ─── La pastille du lecteur ─── -->
<OptionGroup title={$t("guide.badges_title")} hint={$t("guide.badges_hint")}>
  <OptionBlock keywords="{$t('guide.badges_title')} {PASTILLES.map((p) => $t(CHAINE[p].cle)).join(' ')}" class="flex flex-col gap-4 p-4">
    <p class="px-1 text-sm leading-normal text-(--rg-tx2) text-pretty">{$t("guide.badges_intro")}</p>
    <div class="grid grid-cols-2 max-md:grid-cols-1 gap-2.5">
      {#each PASTILLES as p (p)}
        <div class="flex flex-col items-start gap-2 px-4 py-3.5 rounded-xl border border-(--rg-bd) bg-(--rg-creux)">
          <span class="{pastille} {CHAINE[p].teinte}"><span class="w-1.5 h-1.5 rounded-full bg-current"></span>{$t(CHAINE[p].cle)}</span>
          <span class="text-[13px] leading-normal text-(--rg-mu) text-pretty">{$t(CHAINE[p].desc)}</span>
        </div>
      {/each}
    </div>
  </OptionBlock>
</OptionGroup>

<!-- ─── Glossaire ─── -->
<OptionGroup title={$t("guide.glossary_title")} hint={$t("guide.glossary_hint")}>
  <div class="flex flex-wrap gap-1.5 px-5 py-4 border-b border-(--rg-line)">
    {#each TERMES as id (id)}
      <button
        type="button"
        class="h-7 px-3 rounded-full border border-(--rg-bd) bg-(--rg-creux) text-[12.5px] font-semibold text-(--rg-tx2)
               hover:border-(--rg-gbd) hover:text-(--rg-gtx) cursor-pointer transition-colors"
        onclick={() => aller(`terme-${id}`)}
      >
        {$t(`glossary.${id}.name`)}
      </button>
    {/each}
  </div>
  {#each TERMES as id (id)}
    <OptionBlock keywords="{$t(`glossary.${id}.name`)} {texteBrut($t(`glossary.${id}.long`))}">
      <div
        id="terme-{id}"
        class="grid grid-cols-[170px_minmax(0,1fr)] max-md:grid-cols-1 gap-x-6 gap-y-1.5 px-5 py-4.5 transition-colors duration-500
               {cible === `terme-${id}` ? 'bg-(--rg-creux-on) shadow-[inset_3px_0_0_var(--rg-g)]' : ''}"
      >
        <p class="text-[15px] font-bold leading-snug text-(--rg-tx)">{$t(`glossary.${id}.name`)}</p>
        <p class="text-sm leading-[1.6] text-(--rg-tx2) text-pretty"><TexteAide texte={$t(`glossary.${id}.long`)} /></p>
      </div>
    </OptionBlock>
  {/each}
</OptionGroup>

</div>
