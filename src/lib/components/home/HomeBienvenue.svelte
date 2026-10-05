<script lang="ts">
// Accueil sans bibliothèque.
import Icon from "@iconify/svelte";
import DisqueFiligrane from "#lib/components/ui/deco/DisqueFiligrane.svelte";
import { goto } from "$app/navigation";
import { t } from "#lib/i18n";
import { handleClickOpenDirectory } from "#lib/actions/player/PlayerAction";
import { ONGLETS_BIBLIOTHEQUE } from "#lib/config/libraryTabs";

const sections = ONGLETS_BIBLIOTHEQUE.filter((o) => o.key !== "playlists");

const etapes = [
  { icone: "material-symbols:create-new-folder-outline-rounded", titre: "home_welcome.step1_title", desc: "home_welcome.step1_desc" },
  { icone: "material-symbols:auto-awesome-outline-rounded", titre: "home_welcome.step2_title", desc: "home_welcome.step2_desc" },
  { icone: "material-symbols:headphones-outline-rounded", titre: "home_welcome.step3_title", desc: "home_welcome.step3_desc" },
];
</script>

<!-- ─── Bandeau ─── -->
<section class="relative overflow-hidden rounded-[24px] p-8 md:p-12 text-(--rg-tx) select-none">
  <div class="hero-vert absolute inset-0 pointer-events-none" aria-hidden="true"></div>

  <DisqueFiligrane class="absolute -right-20 -bottom-28 w-[440px] h-[440px] max-md:hidden" />

  <div class="relative flex flex-col gap-4 max-w-2xl">
    <p class="text-[13px] font-semibold uppercase tracking-[0.08em] text-(--rg-gtx)">{$t("home_welcome.kicker")}</p>
    <h2 class="text-[36px] md:text-[44px] font-extrabold tracking-[-0.03em] leading-[1.05]">{$t("home_welcome.title")}</h2>
    <p class="text-[16px] leading-relaxed text-(--rg-tx2) max-w-xl">{$t("home_welcome.desc")}</p>

    <div class="flex flex-wrap items-center gap-3 mt-3">
      <button type="button" onclick={() => goto("/import")}
              class="cta h-[52px] px-[26px] rounded-full font-bold text-[17px] flex items-center gap-2.5 cursor-pointer
                     bg-(--rg-g) text-(--rg-on-g)">
        <Icon icon="material-symbols:library-add-outline-rounded" width="22" />
        {$t("home_welcome.start")}
        <Icon icon="material-symbols:arrow-forward-rounded" width="20" class="fleche" />
      </button>
      <button type="button" onclick={() => handleClickOpenDirectory()}
              class="h-[52px] px-5 rounded-full flex items-center gap-2.5 cursor-pointer transition-colors
                     text-(--rg-tx2) hover:text-(--rg-tx) hover:bg-(--rg-carte)/60">
        <Icon icon="material-symbols:play-circle-outline-rounded" width="22" />
        <span class="flex flex-col items-start leading-tight">
          <b class="text-[14px] font-semibold">{$t("home_welcome.just_play")}</b>
          <small class="text-[11.5px] text-(--rg-mu)">{$t("home_welcome.just_play_hint")}</small>
        </span>
      </button>
    </div>
  </div>
</section>

<!-- ─── Les trois étapes ─── -->
<ol class="grid grid-cols-1 @3xl:grid-cols-3 gap-3 select-none">
  {#each etapes as e, i (e.titre)}
    <li class="relative flex gap-4 p-5 rounded-2xl bg-(--rg-creux) border border-(--rg-line)">
      <span class="relative w-11 h-11 shrink-0 rounded-xl flex items-center justify-center bg-(--rg-gbg) text-(--rg-g)">
        <Icon icon={e.icone} width="22" />
        <span class="absolute -top-1.5 -left-1.5 w-5 h-5 rounded-full flex items-center justify-center
                     text-[11px] font-extrabold bg-(--rg-g) text-(--rg-on-g)">{i + 1}</span>
      </span>
      <span class="flex flex-col gap-0.5 min-w-0">
        <b class="text-[15px] font-bold text-(--rg-tx)">{$t(e.titre)}</b>
        <span class="text-[13px] leading-snug text-(--rg-mu)">{$t(e.desc)}</span>
      </span>
    </li>
  {/each}
</ol>

<!-- ─── Sections à venir ─── -->
<section class="flex flex-col gap-3 select-none">
  <h3 class="text-[13px] font-bold uppercase tracking-[0.08em] text-(--rg-mu2)">{$t("home_welcome.explore")}</h3>
  <div class="grid grid-cols-2 @2xl:grid-cols-3 @5xl:grid-cols-6 gap-3">
    {#each sections as s, i (s.key)}
      <button type="button" onclick={() => goto(`/import?section=${s.key}`)}
              class="fantome group flex flex-col gap-3 p-4 rounded-2xl text-left cursor-pointer
                     bg-(--rg-carte) border border-(--rg-bd) hover:border-(--rg-g)">
        <span class="flex items-center gap-2 text-(--rg-tx2) group-hover:text-(--rg-g) transition-colors">
          <Icon icon={s.icon} width="20" />
          <b class="text-[14px] font-semibold">{$t(s.labelKey)}</b>
        </span>
        <span class="flex flex-col gap-1.5" aria-hidden="true">
          <span class="h-2 rounded-full bg-(--rg-s2)" style:width="{88 - (i % 3) * 12}%"></span>
          <span class="h-2 rounded-full bg-(--rg-s2)" style:width="{60 + (i % 2) * 18}%"></span>
          <span class="h-2 rounded-full bg-(--rg-s2) w-2/5"></span>
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .cta {
    transition: transform 160ms ease, box-shadow 160ms ease, filter 160ms ease;
  }
  .cta:hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
    box-shadow: 0 12px 28px -14px rgba(0, 0, 0, 0.5);
  }
  .cta :global(.fleche) {
    transition: transform 160ms ease;
  }
  .cta:hover :global(.fleche) {
    transform: translateX(3px);
  }
  .fantome {
    transition: transform 160ms ease, border-color 160ms ease;
  }
  .fantome:hover {
    transform: translateY(-2px);
  }
  @media (prefers-reduced-motion: reduce) {
    .cta:hover, .fantome:hover, .cta:hover :global(.fleche) { transform: none; }
  }
</style>
