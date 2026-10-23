<script lang="ts">
  // Résultat présent en local et sur un serveur : un disque et un nuage, chacun ouvre sa version.
  // La première est la préférée (celle du clic sur le résultat) : pleine ; l'autre, pâle.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import type { SearchResult, Version } from "#lib/helper/search/searchVersions";

  let { versions, onopen, size = 13 }: { versions: Version[]; onopen: (r: SearchResult) => void; size?: number } = $props();
</script>

{#if versions.length > 1}
  <span class="shrink-0 inline-flex items-center gap-0.5 align-[-2px]">
    {#each versions as v, i (v.remote)}
      <span role="button" tabindex="-1" title={$t(`sources.${v.result.result_type === "track" ? "play" : "open"}_${v.remote ? "server" : "local"}`)}
            class="inline-flex p-0.5 rounded cursor-pointer transition-colors hover:bg-neutral-200/70 dark:hover:bg-white/10
                   {v.remote ? 'text-sky-600 dark:text-sky-400' : 'text-green-600 dark:text-green-400'} {i > 0 ? 'opacity-55 hover:opacity-100' : ''}"
            onclick={(e) => { e.stopPropagation(); onopen(v.result); }}
            onkeydown={(e) => { if (e.key === "Enter") { e.stopPropagation(); onopen(v.result); } }}>
        <Icon icon={v.remote ? "material-symbols:cloud-outline-rounded" : "material-symbols:hard-drive-outline"} width={size} />
      </span>
    {/each}
  </span>
{/if}
