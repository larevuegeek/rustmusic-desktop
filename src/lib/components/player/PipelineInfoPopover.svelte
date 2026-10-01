<script lang="ts">
  // Détail de la chaîne audio, du fichier au DAC, dans le style du menu Sortie.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "$lib/i18n";
  import { pipelineMode, type PlaybackPipelineInfo } from "$lib/stores/player/playbackPipeline.store";
  import { CHAINE } from "$lib/helper/audio/chaineAudio";
  import { decrireSortie } from "$lib/helper/audio/deviceLabel";

  let { info }: { info: PlaybackPipelineInfo } = $props();

  const nombre = $derived(new Intl.NumberFormat($currentLocale, { maximumFractionDigits: 2 }));
  function frequence(hz: number): string {
    if (hz >= 1_000_000) return `${nombre.format(hz / 1_000_000)} MHz`;
    return hz >= 1_000 ? `${nombre.format(hz / 1_000)} kHz` : `${hz} Hz`;
  }

  const CANAUX: Record<number, string> = { 3: "3.0", 4: "4.0", 5: "5.0", 6: "5.1", 7: "6.1", 8: "7.1" };
  function canaux(n: number): string {
    if (n === 1) return $t("pipeline.channels_mono");
    if (n === 2) return $t("pipeline.channels_stereo");
    return CANAUX[n] ?? `${n} ch`;
  }

  const profil = $derived(
    ["high", "medium", "low", "minimal"].includes(info.quality_profile) ? $t(`settings.audio_quality_${info.quality_profile}`) : info.quality_profile,
  );

  const etat = $derived.by(() => {
    const m = pipelineMode(info);
    return m ? CHAINE[m] : null;
  });
  const dsd = $derived(info.intermediate_pcm_rate != null);
  const dop = $derived((info.backend ?? "").endsWith("DoP"));
  const moteur = $derived(dop ? $t("pipeline.dop_backend") : info.backend ?? "CPAL shared");

  type Etape = { icone: string; titre: string; valeur: string; detail?: string; mono?: boolean; ton?: string };
  const AMBRE = "bg-amber-500/14 text-amber-700 dark:text-amber-300";
  const VIOLET = "bg-violet-500/14 text-violet-700 dark:text-violet-300";

  const etapes = $derived.by(() => {
    const e: Etape[] = [{
      icone: "material-symbols-light:audio-file-outline-rounded",
      titre: $t("pipeline.source"),
      valeur: [info.source_format, frequence(info.source_sample_rate), dsd || dop ? "1 bit" : info.source_bits > 0 ? `${info.source_bits} bit` : null].filter(Boolean).join(" · "),
      detail: canaux(info.source_channels),
      mono: true,
    }];
    if (dsd && info.intermediate_pcm_rate) {
      e.push({
        icone: "material-symbols-light:tune-rounded",
        titre: $t("pipeline.decoding"),
        valeur: `DSD → PCM ${frequence(info.intermediate_pcm_rate)}`,
        detail: [info.dsd_filter_taps ? $t("pipeline.dsd_filter").replace("{taps}", String(info.dsd_filter_taps)) : null, info.dsd_decimation ? `×${info.dsd_decimation}` : null].filter(Boolean).join(" · ") || undefined,
        mono: true,
        ton: AMBRE,
      });
    }
    if (dop) {
      e.push({
        icone: "material-symbols-light:swap-horiz-rounded",
        titre: $t("pipeline.transport"),
        valeur: `DoP ${frequence(info.output_sample_rate)}`,
        detail: $t("pipeline.dop_transport_hint"),
        mono: true,
        ton: VIOLET,
      });
    }
    if (info.resampler_active) {
      e.push({
        icone: "material-symbols-light:graphic-eq-rounded",
        titre: $t("pipeline.resampling"),
        valeur: `${frequence(info.intermediate_pcm_rate ?? info.source_sample_rate)} → ${frequence(info.output_sample_rate)}`,
        mono: true,
        ton: AMBRE,
      });
    }
    e.push({
      icone: "material-symbols-light:speaker-outline-rounded",
      titre: $t("pipeline.output"),
      valeur: decrireSortie({ name: info.device_name }).nom,
      detail: [frequence(info.output_sample_rate), canaux(info.output_channels), moteur].join(" · "),
    });
    return e;
  });
</script>

<div class="w-80 p-1.5 rounded-[14px] border text-left bg-(--lc-menu) border-(--lc-menu-bd) shadow-[0_18px_40px_rgba(0,0,0,0.35)] text-(--lc-tx)">
  <div class="flex items-center justify-between gap-2 px-2.5 pt-1.5 pb-1.5">
    <p class="text-[11px] font-bold uppercase tracking-[0.08em] text-(--lc-mu)">{$t("player.audio_chain")}</p>
    {#if etat}
      <span class="h-5.5 px-2 flex items-center gap-1.5 rounded-full border text-[10.5px] font-semibold whitespace-nowrap {etat.teinte}">
        <span class="w-1.5 h-1.5 rounded-full bg-current"></span>{$t(etat.cle)}
      </span>
    {/if}
  </div>
  {#if etat}<p class="px-2.5 pb-2.5 text-[11.5px] leading-snug text-(--lc-mu)">{$t(etat.desc)}</p>{/if}

  <!-- Étapes reliées, du fichier au DAC. -->
  <ol class="px-2.5 pt-1">
    {#each etapes as e, i (e.titre)}
      <li class="relative flex gap-3 pb-3">
        {#if i < etapes.length - 1}<span class="absolute left-3.25 top-7.5 -bottom-0.5 w-px bg-(--lc-menu-bd)"></span>{/if}
        <span class="shrink-0 w-6.5 h-6.5 rounded-lg flex items-center justify-center {e.ton ?? 'bg-(--lc-survol) text-(--lc-tx2)'}">
          <Icon icon={e.icone} width="17" />
        </span>
        <span class="min-w-0 flex-1 leading-[1.3]">
          <span class="block text-[10px] font-bold uppercase tracking-[0.07em] text-(--lc-mu)">{e.titre}</span>
          <span class="block truncate text-[12.5px] font-semibold {e.mono ? 'font-mono font-medium text-[12px]' : ''}" title={e.valeur}>{e.valeur}</span>
          {#if e.detail}<span class="block text-[11px] text-(--lc-mu)">{e.detail}</span>{/if}
        </span>
      </li>
    {/each}
  </ol>

  {#if profil}
    <div class="flex items-center justify-between px-2.5 pt-2 pb-1.5 border-t border-(--lc-menu-bd) text-[11.5px]">
      <span class="text-(--lc-mu)">{$t("pipeline.profile")}</span>
      <span class="font-semibold text-(--lc-tx2)">{profil}</span>
    </div>
  {/if}
</div>
