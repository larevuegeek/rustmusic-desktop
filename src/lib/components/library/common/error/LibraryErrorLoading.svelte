<script lang="ts">
  import { t } from "$lib/i18n";
import Icon from "@iconify/svelte";

let { error }: { error: string } = $props();
    
</script>
<div class="w-full h-full flex items-center justify-center py-20">

<div class="flex flex-col items-center gap-5 
            px-8 py-6 rounded-2xl
            backdrop-blur-md
            bg-white/60 dark:bg-zinc-900/60
            border border-red-500/20 dark:border-red-500/30
            shadow-[0_10px_40px_-15px_rgba(0,0,0,0.5)]
            max-w-md text-center">

    <!-- Icon -->
    <div class="w-14 h-14 flex items-center justify-center 
                rounded-full bg-red-500/10 text-red-500">
    <Icon icon="mynaui:warning-triangle" class="w-7 h-7" />
    </div>

    <!-- Title -->
    <div class="text-base font-semibold text-neutral-800 dark:text-neutral-200">
    {$t("common.error_library")}
    </div>

    <!-- Error message -->
    <div class="text-sm text-neutral-600 dark:text-neutral-400">
    {error}
    </div>

    <!-- Action -->
    <button
    onclick={() => location.reload()}
    class="mt-2 px-4 py-2 rounded-lg text-sm font-medium
            bg-red-500 text-white
            hover:bg-red-600
            active:scale-[0.98]
            transition-all duration-200">
    {$t("common.retry")}
    </button>
</div>
</div>