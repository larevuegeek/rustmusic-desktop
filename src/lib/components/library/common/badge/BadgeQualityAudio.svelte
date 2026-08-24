<script lang="ts">
import { dsdLabel, formatChannels, formatDsdRate, isDsdFormat } from "$lib/helper/tools/audioFormatTools";

let { track = null, size = "md" }: { track?: any; size?: Size } = $props();

type Size = "sm" | "md" | "lg" | "xl";

type QualityType = "dsd" | "hires" | "lossless" | "high" | "medium" | "low" | "unknown";

  interface Badge {
    label: string;
    type: QualityType;
    tooltip: string;
  }

  function hzToKhz(hz: number | null): string {
    if (!hz) return "? kHz";
    const khz = hz / 1000;
    // 44.1 / 48 / 96 / 192 -> on garde 1 décimale si besoin
    const str = Number.isInteger(khz) ? `${khz}` : `${khz.toFixed(1)}`;
    return `${str} kHz`;
  }

  function bpsToKbps(bps: number | null): string {
    if (!bps) return "? kbps";
    return `${Math.round(bps / 1000)} kbps`;
  }

  function computeBadge(t: any): Badge | null {
    if (!t) return null;

    const fmt = t.audio_format?.toLowerCase();
    const bitDepth = t.bits_per_sample;
    const sr = t.sample_rate;
    const br = t.bitrate;

    // 0) DSD (DSF / DFF) — niveau premium au-dessus du Hi-Res classique
    if (isDsdFormat(t.audio_format)) {
      const label = dsdLabel(sr);
      return {
        label,
        type: "dsd",
        tooltip: `${label} • ${formatDsdRate(sr)} • ${formatChannels(t.channels)}`,
      };
    }

    // 1) Hi-Res / Lossless (plutôt basé sur bit depth + format)
    // Formats souvent lossless : flac, wav, alac, aiff, ape, wv...
    const isLikelyLossless =
      fmt === "flac" ||
      fmt === "wav" ||
      fmt === "alac" ||
      fmt === "aiff" ||
      fmt === "ape" ||
      fmt === "wv";

    if (isLikelyLossless && bitDepth && bitDepth >= 24) {
      return {
        label: `Hi-Res ${bitDepth}-bit`,
        type: "hires",
        tooltip: `${fmt?.toUpperCase() ?? "AUDIO"} • ${bitDepth}-bit • ${hzToKhz(sr)} • ${t.channels ?? "?"} ch`,
      };
    }

    if (isLikelyLossless && bitDepth === 16) {
      return {
        label: "Lossless 16-bit",
        type: "lossless",
        tooltip: `${fmt?.toUpperCase() ?? "AUDIO"} • 16-bit • ${hzToKhz(sr)} • ${t.channels ?? "?"} ch`,
      };
    }

    // 2) Lossy (MP3/AAC/OGG/OPUS… -> on raisonne surtout sur bitrate)
    const isLikelyLossy =
      fmt === "mp3" ||
      fmt === "aac" ||
      fmt === "m4a" ||
      fmt === "ogg" ||
      fmt === "opus";

    if (isLikelyLossy && br) {
      const kbps = Math.round(br / 1000);
      const labelFmt = fmt === "m4a" ? "AAC" : fmt.toUpperCase();

      if (kbps >= 320) {
        return { label: `${labelFmt} ${kbps}`, type: "high", tooltip: `${labelFmt} • ${kbps} kbps` };
      }
      if (kbps >= 256) {
        return { label: `${labelFmt} ${kbps}`, type: "medium", tooltip: `${labelFmt} • ${kbps} kbps` };
      }
      if (kbps >= 192) {
        return { label: `${labelFmt} ${kbps}`, type: "low", tooltip: `${labelFmt} • ${kbps} kbps` };
      }

      return { label: `${labelFmt} low`, type: "low", tooltip: `${labelFmt} • ${kbps} kbps` };
    }

    // 3) Cas “pas d’info” : on peut afficher un badge simple basé sur format
    if (fmt) {
      return {
        label: fmt.toUpperCase(),
        type: "unknown",
        tooltip: [
          fmt.toUpperCase(),
          bitDepth ? `${bitDepth} bit` : null,
          sr ? hzToKhz(sr) : null,
          br ? bpsToKbps(br) : null,
        ]
          .filter(Boolean)
          .join(" • "),
      };
    }

    return null;
  }

    function classes(type: QualityType): string {
    switch (type) {
        case "dsd":
        // Style premium DSD : ambre saturé + halo + typo bold tracking
        return `
            bg-amber-500/12
            text-amber-200
            font-bold tracking-wider
            border border-amber-400/45
            shadow-[0_0_14px_rgba(251,191,36,0.28)]
        `;

        case "hires":
        return `
            bg-emerald-400/10
            text-emerald-300
            border border-emerald-400/25
            shadow-[0_0_8px_rgba(16,185,129,0.15)]
        `;

        case "lossless":
        return `
            bg-blue-400/5
            text-blue-300/90
            border border-blue-400/15
        `;

        case "high":
        return `
            bg-white/5
            text-white/80
            border border-white/10
        `;

        case "medium":
        return `
            bg-white/4
            text-white/65
            border border-white/8
        `;

        case "low":
        return `
            bg-white/3
            text-white/50
            border border-white/6
        `;

        default:
        return `
            bg-white/4
            text-white/60
            border border-white/8
        `;
    }
    }

  function classesSize(size: Size): string {
    // cohérent avec ton style dark (border léger + fond translucide)
    switch (size) {
      case "md":
        return "py-1 rounded-md text-[10px] font-semibold";
      case "sm":
        return "rounded-sm text-[10px] font-semibold";
      default:
        return "py-1 rounded-md text-[10px] font-semibold";
    }
  }
  

  let badge = $derived(computeBadge(track));
</script>

{#if badge}
  <span
    class={`inline-flex items-center px-2 tracking-wide
        ${classes(
        badge.type
        )}
        ${classesSize(
        size
        )}
    `}
    title={badge.tooltip}
  >
    {badge.label}
  </span>
{/if}