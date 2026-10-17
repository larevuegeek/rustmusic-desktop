<script lang="ts">
  // Pastille « Réseau » de la barre du lecteur : ses icônes disent ce qui est actif,
  // son menu permet de tout basculer sans passer par les réglages.
  import { goto } from "$app/navigation";
  import Icon from "@iconify/svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import { t } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { networkPlayerStore } from "#lib/stores/audio/networkPlayer.store";
  import { dlnaStatusStore } from "#lib/stores/dlna/dlna.store";
  import { dlnaStart, dlnaStop } from "#lib/services/dlna/dlna.service";
  import { TEINTE } from "#lib/helper/audio/audioChain";

  let { pastille, voyant, cliquable }: { pastille: string; voyant: string; cliquable: string } = $props();

  let ouvert = $state(false);
  let menuEl: HTMLElement | null = $state(null);
  let dlnaBascule = $state(false);

  const lecteurs = $derived($settingsStore.network_players === "true");
  const airplay = $derived($settingsStore.airplay === "true");
  const partage = $derived($dlnaStatusStore?.running ?? false);
  const choisi = $derived($networkPlayerStore.selected);

  const ICONE_LECTEURS = "material-symbols-light:cast-rounded";
  const ICONE_AIRPLAY = "material-symbols-light:airplay-rounded";
  const ICONE_PARTAGE = "material-symbols-light:folder-shared-outline-rounded";

  // Détail au survol, une ligne par service actif.
  const resume = $derived(
    [
      lecteurs && $t("network_player.pill_upnp"),
      airplay && $t("network_player.pill_airplay"),
      partage && `${$t("network_player.pill_dlna")}${$dlnaStatusStore?.url ? `\n${$dlnaStatusStore.url}` : ""}`,
    ]
      .filter(Boolean)
      .join("\n"),
  );

  /** État d'une famille de lecteurs : lecture en cours dessus, activée, ou coupée. */
  function statusLabel(kind: "upnp" | "airplay", actif: boolean): string {
    if (!actif) return $t("network_player.menu_off");
    if (choisi?.kind === kind) return $t("network_player.menu_playing_on").replace("{name}", choisi.name);
    return $t("network_player.menu_ready");
  }

  async function toggleSharing() {
    if (dlnaBascule) return;
    dlnaBascule = true;
    try {
      dlnaStatusStore.set(partage ? await dlnaStop() : await dlnaStart());
    } catch (e) {
      console.error("[dlna] bascule :", e);
    } finally {
      dlnaBascule = false;
    }
  }

  // Fermeture au clic extérieur.
  $effect(() => {
    if (!ouvert) return;
    const handleOutside = (e: MouseEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) ouvert = false;
    };
    const timer = setTimeout(() => document.addEventListener("click", handleOutside), 50);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("click", handleOutside);
    };
  });
</script>

<!-- Visible si un service tourne ; reste ouvert le temps de tout couper depuis le menu. -->
{#if lecteurs || airplay || partage || ouvert}
  <!-- Sans `relative` : le menu se cale sur le bord haut de la carte du lecteur. -->
  <div bind:this={menuEl}>
    <button
      type="button"
      class="{pastille} {TEINTE.vert} {cliquable} {ouvert ? 'border-current/50' : ''}"
      title={resume}
      aria-expanded={ouvert}
      onclick={() => (ouvert = !ouvert)}
    >
      {#if lecteurs}<Icon icon={ICONE_LECTEURS} width="14" class="shrink-0" />{/if}
      {#if airplay}<Icon icon={ICONE_AIRPLAY} width="14" class="shrink-0" />{/if}
      {#if partage}<span class={voyant}></span>{/if}
      {$t("network_player.pill_network")}
    </button>

    {#if ouvert}
      <div
        class="absolute bottom-full right-2 mb-2 z-50 w-96 p-1.5 rounded-[16px] border
               bg-(--lc-menu) border-(--lc-menu-bd) shadow-[0_18px_40px_rgba(0,0,0,0.35)] text-(--lc-tx)"
      >
        <div class="flex items-center gap-2 px-2.5 pt-2 pb-1">
          <Icon icon="material-symbols-light:lan-outline-rounded" width="18" class="shrink-0 text-(--lc-mu)" />
          <p class="flex-1 text-[13px] font-bold">{$t("network_player.pill_network")}</p>
          <Badge tone="amber">{$t("settings.beta_badge")}</Badge>
        </div>

        {#snippet ligne(icone: string, titre: string, statusLabel: string, actif: boolean, vivant: boolean, basculer: () => void, occupe = false)}
          <div
            role="button"
            tabindex="0"
            class="flex items-center gap-3 px-2.5 py-2 rounded-[12px] cursor-pointer transition-colors
                   {vivant ? 'bg-(--rg-g)/10 hover:bg-(--rg-g)/14' : 'hover:bg-(--lc-survol)'}"
            onclick={() => !occupe && basculer()}
            onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), !occupe && basculer())}
          >
            <span class="w-9 h-9 shrink-0 flex items-center justify-center rounded-[10px] transition-colors
                         {actif ? 'bg-(--rg-g)/15 text-(--rg-g)' : 'bg-(--lc-survol) text-(--lc-mu)'}">
              <Icon icon={icone} width="20" />
            </span>
            <span class="min-w-0 flex-1 leading-[1.25]">
              <span class="block truncate text-[13px] font-semibold {actif ? '' : 'text-(--lc-mu)'}">{titre}</span>
              <span class="flex items-center gap-1.5 text-[11.5px] {vivant ? 'text-(--rg-gtx)' : 'text-(--lc-mu)'}">
                {#if vivant}<span class="w-1.5 h-1.5 shrink-0 rounded-full bg-(--rg-g) shadow-[0_0_6px_var(--rg-g)]"></span>{/if}
                <span class="truncate">{statusLabel}</span>
              </span>
            </span>
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="shrink-0" onclick={(e) => e.stopPropagation()}>
              <ToggleSwitch size="sm" checked={actif} disabled={occupe} label={titre} onclick={basculer} />
            </div>
          </div>
        {/snippet}

        <p class="px-2.5 pt-2.5 pb-1 text-[10.5px] font-bold uppercase tracking-[0.08em] text-(--lc-mu)">{$t("network_player.menu_send")}</p>
        {@render ligne(ICONE_LECTEURS, $t("network_player.menu_players"), statusLabel("upnp", lecteurs), lecteurs, choisi?.kind === "upnp", () => settingsStore.toggle("network_players"))}
        {@render ligne(ICONE_AIRPLAY, $t("network_player.menu_airplay"), statusLabel("airplay", airplay), airplay, choisi?.kind === "airplay", () => settingsStore.toggle("airplay"))}

        <p class="px-2.5 pt-2.5 pb-1 text-[10.5px] font-bold uppercase tracking-[0.08em] text-(--lc-mu)">{$t("network_player.menu_share_group")}</p>
        {@render ligne(
          ICONE_PARTAGE,
          $t("network_player.menu_share"),
          partage ? `${$t("network_player.menu_online")} · ${$dlnaStatusStore?.url?.replace(/^https?:\/\//, "").replace(/\/.*$/, "") ?? ""}` : $t("network_player.menu_off"),
          partage,
          partage,
          toggleSharing,
          dlnaBascule,
        )}

        <button
          type="button"
          class="mt-1.5 w-full flex items-center justify-center gap-2 h-9 rounded-[12px] border border-(--lc-menu-bd) cursor-pointer
                 text-[12.5px] font-semibold text-(--lc-mu) hover:text-(--lc-tx) hover:bg-(--lc-survol) transition-colors"
          onclick={() => { ouvert = false; goto("/settings/network"); }}
        >
          <Icon icon="material-symbols-light:settings-outline-rounded" width="17" class="shrink-0" />
          {$t("network_player.menu_settings")}
          <Icon icon="material-symbols-light:arrow-forward-rounded" width="15" class="shrink-0" />
        </button>
      </div>
    {/if}
  </div>
{/if}
