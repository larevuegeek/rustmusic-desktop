<script lang="ts">
  // Titres passés à l'antenne pendant l'écoute : retrouver « ce morceau entendu tout à l'heure ».
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { t, currentLocale } from "#lib/i18n";
  import { ilYA } from "#lib/helper/tools/dateTools";
  import { toasts } from "#lib/stores/ui/toast.store";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import RadioLogo from "./RadioLogo.svelte";

  type RadioTitle = { id: number; station: string; stream_url: string; title: string; played_at: string; logo_path: string | null };

  let { profilId }: { profilId: number } = $props();

  const SHOWN = 12;
  let titles = $state<RadioTitle[]>([]);
  let all = $state(false);
  let copied = $state<number | null>(null);

  async function load() {
    titles = (await invoke<RadioTitle[]>("get_radio_titles", { profilId }).catch(() => null)) ?? titles;
  }

  $effect(() => {
    void profilId;
    void load();
    const off = listen<number>("radio-titles-updated", (e) => {
      if (e.payload === profilId) void load();
    });
    return () => void off.then((u) => u());
  });

  async function clear() {
    await invoke("clear_radio_titles", { profilId }).catch((e) => toasts.push({ type: "error", title: $t("notify.error"), message: String(e) }));
    titles = [];
  }

  async function copy(r: RadioTitle) {
    try {
      await navigator.clipboard.writeText(r.title);
      copied = r.id;
      setTimeout(() => (copied = null), 1500);
    } catch {
      /* presse-papiers indisponible : rien à faire */
    }
  }

  const shown = $derived(all ? titles : titles.slice(0, SHOWN));
</script>

{#if titles.length}
  <section class="mt-12">
    <div class="flex flex-wrap items-end gap-3 mb-3.5">
      <div class="flex flex-col gap-1 min-w-0">
        <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("radios.titles")}</h2>
        <p class="text-[13.5px] text-(--rg-mu)">{$t("radios.titles_hint")}</p>
      </div>
      <span class="flex-1"></span>
      <GhostButton icon="material-symbols:delete-sweep-outline-rounded" label={$t("radios.titles_clear")} onclick={clear} />
    </div>

    <div class="grid grid-cols-1 xl:grid-cols-2 gap-x-6 gap-y-0.5">
      {#each shown as r (r.id)}
        <div class="group flex items-center gap-3 px-2 py-2 rounded-xl transition-colors hover:bg-(--rg-carte)">
          <RadioLogo path={r.logo_path} icone={18} class="w-10 h-10 shrink-0 rounded-lg" />
          <div class="min-w-0 flex-1">
            <p class="truncate text-[14.5px] font-semibold text-(--rg-tx)" title={r.title}>{r.title}</p>
            <p class="truncate text-[12.5px] text-(--rg-mu)">{r.station} · {ilYA(r.played_at, $currentLocale, "short")}</p>
          </div>
          <button type="button" onclick={() => copy(r)} title={$t("radios.titles_copy")} aria-label={$t("radios.titles_copy")}
                  class="w-8.5 h-8.5 shrink-0 flex items-center justify-center rounded-lg cursor-pointer transition-all text-(--rg-mu)
                         opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-(--rg-s2) hover:text-(--rg-tx)">
            <Icon icon={copied === r.id ? "material-symbols:check-rounded" : "material-symbols:content-copy-outline-rounded"} width="17" />
          </button>
        </div>
      {/each}
    </div>

    {#if titles.length > SHOWN}
      <div class="mt-3 flex justify-center">
        <GhostButton icon={all ? "material-symbols:expand-less-rounded" : "material-symbols:expand-more-rounded"}
                     label={all ? $t("radios.titles_less") : $t("radios.titles_more").replace("{n}", String(titles.length - SHOWN))}
                     onclick={() => (all = !all)} />
      </div>
    {/if}
  </section>
{/if}
