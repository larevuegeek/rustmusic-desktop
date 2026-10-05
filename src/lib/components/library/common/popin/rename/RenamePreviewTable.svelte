<script lang="ts">
  // L'aperçu à blanc : nom actuel → nom après, une case par ligne modifiable.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import type { RenamePreview } from "#lib/services/tags/rename.service";

  const {
    empty,
    preview,
    excluded,
    selectedCount,
    changeableCount,
    allSelected,
    ontoggle,
    onselectall,
  }: {
    /** Aucun fichier ciblé. */
    empty: boolean;
    preview: RenamePreview | null;
    excluded: Set<string>;
    selectedCount: number;
    changeableCount: number;
    allSelected: boolean;
    ontoggle: (path: string) => void;
    onselectall: (all: boolean) => void;
  } = $props();
</script>

<div class="flex-1 min-h-0 overflow-y-auto scrollbar-none px-3 py-2.5">
  {#if empty}
    <p class="py-16 text-center text-[12.5px] text-neutral-400 dark:text-neutral-500">
      {$t("rename.no_file")}
    </p>
  {:else if !preview}
    <div class="space-y-1" aria-hidden="true">
      {#each Array.from({ length: 6 }) as _, i (i)}
        <div class="h-9 rounded-lg animate-pulse bg-neutral-100/70 dark:bg-white/4"></div>
      {/each}
    </div>
  {:else}
    <!-- Colonnes nommées : savoir lequel des deux noms est l'actuel. -->
    <div class="flex items-center gap-2.5 px-2.5 pb-1.5">
      <!-- État intermédiaire : trois lignes décochées sur cent, une case vide mentirait. -->
      <input
        type="checkbox"
        checked={allSelected}
        indeterminate={selectedCount > 0 && !allSelected}
        disabled={changeableCount === 0}
        onchange={() => onselectall(!allSelected)}
        aria-label={$t("source.select")}
        class="checkbox-app"
      />
      <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase tracking-widest
                   text-neutral-400 dark:text-neutral-500">
        {$t("rename.current")}
      </span>
      <span class="shrink-0 w-3"></span>
      <span class="flex-1 min-w-0 text-[9.5px] font-semibold uppercase tracking-widest
                   text-emerald-600/70 dark:text-emerald-400/70">
        {$t("rename.after")}
      </span>
    </div>

    <div class="space-y-0.5">
      {#each preview.moves as move (move.from)}
        {@const on = !move.blocked && !move.unchanged && !excluded.has(move.from)}
        <div
          class="flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg
                 {move.blocked
                   ? 'bg-red-500/8'
                   : move.unchanged
                     ? 'bg-neutral-100/70 dark:bg-white/3 opacity-55'
                     : on
                       ? 'bg-emerald-500/8'
                       : 'bg-neutral-100/70 dark:bg-white/3'}"
        >
          <!-- Pas de case sans choix : bloquée ou déjà conforme, rien à cocher. -->
          {#if move.blocked || move.unchanged}
            <span class="shrink-0 w-4 flex items-center justify-center">
              <Icon
                icon={move.blocked ? "lucide:ban" : "lucide:check"}
                width="11"
                class={move.blocked
                  ? "text-red-500/70"
                  : "text-neutral-300 dark:text-neutral-600"}
              />
            </span>
          {:else}
            <input
              type="checkbox"
              checked={on}
              onchange={() => ontoggle(move.from)}
              aria-label={move.from_name}
              class="checkbox-app"
            />
          {/if}

          <span class="flex-1 min-w-0 text-[12px] truncate
                       {on || move.blocked
                         ? 'text-neutral-600 dark:text-neutral-300'
                         : 'text-neutral-400 dark:text-neutral-500'}"
                title={move.from}>
            {move.from_name}
          </span>

          <Icon
            icon={move.blocked
              ? "lucide:x"
              : move.unchanged
                ? "lucide:equal"
                : "lucide:arrow-right"}
            width="11"
            class="shrink-0 {move.blocked
              ? 'text-red-500'
              : 'text-neutral-300 dark:text-neutral-600'}"
          />

          <span class="flex-1 min-w-0 text-[12px] truncate" title={move.to}>
            {#if move.blocked}
              <!-- La raison plutôt que le nom cible : elle dit quoi changer. -->
              <span class="text-red-500">
                {move.issues.map((i) => $t(`rename.issue.${i}`)).join(" · ")}
                {#if move.missing.length > 0}
                  : {move.missing.map((f) => $t(`rename.field.${f}`)).join(", ")}
                {/if}
              </span>
            {:else if move.unchanged}
              <span class="italic text-neutral-400 dark:text-neutral-500">
                {$t("rename.unchanged")}
              </span>
            {:else}
              <span class={on
                ? "font-medium text-emerald-600 dark:text-emerald-400"
                : "text-neutral-400 dark:text-neutral-500"}>
                {move.to_name}
              </span>
            {/if}
          </span>
        </div>
      {/each}
    </div>
  {/if}
</div>
