<script lang="ts">
  import Icon from "@iconify/svelte";
  import { activeTasks, taskProgressStore } from "#lib/stores/ui/taskProgress.store";
  import { slide, fade } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "#lib/i18n";

  async function cancelTask(taskId: string) {
    try {
      await invoke('cancel_task', { taskId });
    } catch {
      // Fallback: on retire juste la tâche côté frontend
    }
    taskProgressStore.cancel(taskId);
  }
</script>

{#if $activeTasks.length > 0}
  <div class="flex items-center gap-2.5 ml-auto shrink-0" transition:slide={{ axis: 'x', duration: 250 }}>
    <span class="h-3 w-px bg-neutral-300/50 dark:bg-neutral-700/40"></span>

    {#each $activeTasks as task (task.id)}
      <div
        transition:fade={{ duration: 200 }}
        class="group/task flex items-center gap-2 pl-2 pr-2.5 py-1 rounded-lg transition-all duration-500
               {task.active
                 ? 'bg-emerald-500/8 dark:bg-emerald-400/8 ring-1 ring-emerald-500/15 dark:ring-emerald-400/10'
                 : 'bg-neutral-100/60 dark:bg-white/4'}"
      >
        <!-- Icône -->
        <div class="relative flex items-center justify-center w-4 h-4">
          {#if task.active}
            <div class="absolute inset-0 rounded-full bg-emerald-500/15 dark:bg-emerald-400/15 animate-ping" style="animation-duration: 2s;"></div>
            <Icon icon={task.icon || "lucide:activity"} width={12}
                  class="relative text-emerald-600 dark:text-emerald-400" />
          {:else}
            <Icon icon="lucide:check-circle-2" width={12}
                  class="text-emerald-500 dark:text-emerald-400" />
          {/if}
        </div>

        <!-- Infos -->
        <div class="flex items-center gap-1.5">
          <span class="text-[10px] font-medium whitespace-nowrap
                       {task.active
                         ? 'text-emerald-700 dark:text-emerald-300'
                         : 'text-neutral-400 dark:text-neutral-500'}">
            {task.label}
          </span>

          {#if task.active && task.total > 0}
            <!-- Pourcentage -->
            <span class="text-[10px] font-semibold tabular-nums text-emerald-600/80 dark:text-emerald-400/70">
              {task.percent}%
            </span>

            <!-- Barre de progression premium -->
            <div class="w-16 h-1.5 rounded-full bg-neutral-200/70 dark:bg-white/8 overflow-hidden">
              <div
                class="h-full rounded-full relative overflow-hidden transition-all duration-500 ease-out
                       bg-linear-to-r from-emerald-500 to-emerald-400 dark:from-emerald-400 dark:to-emerald-300"
                style="width: {task.percent}%"
              >
                <div class="absolute inset-0 bg-linear-to-r from-transparent via-white/30 to-transparent
                            animate-shimmer"></div>
              </div>
            </div>

            <!-- Détail (nom du fichier tronqué) -->
            {#if task.detail}
              <span class="text-[9px] text-neutral-400 dark:text-neutral-500 truncate max-w-24">
                {task.detail}
              </span>
            {/if}
          {/if}
        </div>

        <!-- Bouton annuler (visible au hover) -->
        {#if task.active}
          <button
            onclick={() => cancelTask(task.id)}
            class="opacity-0 group-hover/task:opacity-100 transition-opacity duration-200
                   p-0.5 rounded hover:bg-red-500/15 text-neutral-400 hover:text-red-500
                   dark:text-neutral-500 dark:hover:text-red-400"
            title={$t("common.cancel")}
          >
            <Icon icon="lucide:x" width={10} />
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(200%); }
  }
  .animate-shimmer {
    animation: shimmer 1.5s ease-in-out infinite;
  }
</style>
