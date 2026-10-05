<script lang="ts">
import { goto } from "$app/navigation";
import Title from "#lib/components/ui/text/Title.svelte";
import Icon from "@iconify/svelte";
import { libraryStore } from "#lib/stores/library/library.store";
import { popinStore } from "#lib/stores/ui/popin.store";
import AddLibraryPopin from "#lib/components/library/common/popin/AddLibraryPopin.svelte";
import { t, currentLocale } from "#lib/i18n";
</script>

<div class="p-10 flex flex-col gap-8">

  <Title title={$t("home.my_libraries")} />

  {#if $libraryStore.libraries.length === 0}

    <!-- EMPTY STATE -->
    <div class="flex flex-col items-center justify-center py-20 text-center">

      <div class="w-16 h-16 rounded-2xl 
                  bg-neutral-200 dark:bg-neutral-800
                  flex items-center justify-center
                  text-neutral-400 dark:text-neutral-500 mb-5">
        <Icon icon="lucide:library" width={28} />
      </div>

      <p class="text-lg font-medium text-neutral-700 dark:text-neutral-300">
        {$t("library.no_library")}
      </p>

      <p class="mt-2 text-sm text-neutral-500 dark:text-neutral-400 max-w-md">
        {$t("home.no_library_desc")}
      </p>

        <button
        class="mt-6 inline-flex items-center gap-2
                px-5 py-2.5
                rounded-full
                text-sm font-medium
                bg-linear-to-r from-green-500 to-green-600
                text-white
                hover:from-green-600 hover:to-green-700
                active:scale-[0.96]
                transition-all duration-200
                cursor-pointer"
        onclick={() =>
            popinStore.open(
            $t("library.create_library"),
            AddLibraryPopin,
            {}
            )
        }
        >
            <Icon icon="lucide:plus" width={16} />
            {$t("home.new_library")}
        </button>
    </div>
  {:else}

    <!-- GRID LIST -->
    <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-6">

      {#each $libraryStore.libraries as library (library.id)}

        <button
          type="button"
          onclick={() => goto(`/library/${library.id}`)}
          class="group flex flex-col p-5 rounded-xl
                 bg-neutral-100 dark:bg-neutral-900
                 hover:bg-neutral-200 dark:hover:bg-neutral-800
                 transition-all duration-200
                 shadow-sm hover:shadow-md
                 text-left cursor-pointer"
        >

          <!-- HEADER ROW -->
          <div class="flex items-center justify-between mb-3">

            <div class="w-10 h-10 rounded-lg
                        bg-linear-to-br from-cyan-500/15 to-blue-600/25
                        flex items-center justify-center
                        text-blue-400">
              <Icon icon="lucide:library" width={18} />
            </div>

            <Icon icon="lucide:chevron-right" 
                  class="opacity-0 group-hover:opacity-60 transition-opacity"
                  width={18} />
          </div>

          <!-- TITLE -->
        <span class="font-semibold text-neutral-800 dark:text-neutral-200 truncate">
        {library.name}
        </span>

        <!-- META -->
        <span class="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
        {library.total_tracks.toLocaleString($currentLocale)}
        {library.total_tracks > 1 ? $t("library_head.tracks_n") : $t("library_head.tracks_one")}
        </span>

        {#if library.description}
        <span class="text-xs text-neutral-400 dark:text-neutral-500 truncate mt-1">
            {library.description}
        </span>
        {/if}

        </button>

      {/each}
    </div>
  {/if}
</div>
