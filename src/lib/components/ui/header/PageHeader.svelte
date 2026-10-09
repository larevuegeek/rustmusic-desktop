<script lang="ts">
  import { t, currentLocale } from "#lib/i18n";
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";

  let {
    title,
    subtitle = "",
    icon = "lucide:music",
    iconColor = "var(--color-accent-500)",
    count = 0,
    actions,
    extra,
  }: {
    title: string;
    subtitle?: string;
    icon?: string;
    iconColor?: string;
    count?: number;
    actions?: Snippet;
    extra?: Snippet;
  } = $props();
</script>

<div class="flex items-end gap-6 mb-8">
  <!-- Icône -->
  <div class="w-28 h-28 rounded-2xl shrink-0 flex items-center justify-center"
       style="background: linear-gradient(135deg, {iconColor}15, {iconColor}25);
              border: 1px solid {iconColor}20;
              box-shadow: 0 0 30px {iconColor}10;">
    <Icon icon={icon} width="40" height="40"
          style="color: {iconColor}; filter: drop-shadow(0 0 8px {iconColor}80);" />
  </div>

  <!-- Infos -->
  <div class="flex-1 min-w-0">
    {#if subtitle}
      <p class="text-xs uppercase tracking-widest text-neutral-500 dark:text-neutral-400 mb-1">
        {subtitle}
      </p>
    {/if}

    <h1 class="text-3xl font-bold text-neutral-900 dark:text-neutral-100">
      {title}
    </h1>

    <div class="flex items-center gap-3 mt-2">
      <p class="text-xs text-neutral-400 dark:text-neutral-500">
        {count.toLocaleString($currentLocale)} {$t(count === 1 ? "library_head.tracks_one" : "library_head.tracks_n")}
      </p>

      {#if actions}
        {@render actions()}
      {/if}
    </div>

    {#if extra}
      <div class="mt-4">
        {@render extra()}
      </div>
    {/if}
  </div>
</div>
