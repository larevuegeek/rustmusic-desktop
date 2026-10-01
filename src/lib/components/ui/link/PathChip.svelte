<script lang="ts">
  // Chemin en police mono, avec « ouvrir le dossier » et « copier ».
  import Icon from "@iconify/svelte";
  import { t } from "$lib/i18n";
  import { copierChemin, revelerDansDossier } from "$lib/helper/tools/fileTools";

  let { chemin, aReveler = chemin }: {
    chemin: string;
    /** Fichier montré dans l'explorateur (par défaut le chemin lui-même). */
    aReveler?: string;
  } = $props();

  const bouton = "w-6 h-6 shrink-0 flex items-center justify-center rounded-md cursor-pointer transition-colors text-(--rg-mu2) hover:bg-(--rg-s2) hover:text-(--rg-tx)";
</script>

<div class="h-7.5 max-w-full min-w-0 pl-1 pr-1.5 flex items-center gap-1 rounded-lg border bg-(--rg-carte) border-(--rg-bd)">
  <button type="button" class={bouton} title={$t("album_view.open_folder")} aria-label={$t("album_view.open_folder")} onclick={() => revelerDansDossier(aReveler)}>
    <Icon icon="material-symbols:folder-outline-rounded" width="16" />
  </button>
  <span class="font-mono text-xs truncate min-w-0 text-(--rg-tx2)" title={chemin}>{chemin}</span>
  <button type="button" class={bouton} title={$t("album_view.copy_path")} aria-label={$t("album_view.copy_path")} onclick={() => copierChemin(chemin)}>
    <Icon icon="material-symbols:content-copy-outline-rounded" width="15" />
  </button>
</div>
