<script lang="ts">
  // Champs groupés (morceau, album, extra) des deux éditeurs de tags, squelette compris.
  // Chaque éditeur fournit le rendu d'un champ et sa ligne piste / disque.
  import type { Snippet } from "svelte";
  import { t } from "$lib/i18n";
  import TagGroupTitle from "./TagGroupTitle.svelte";

  let {
    loading,
    field,
    numbers,
  }: {
    loading: boolean;
    /** Un champ : clé, clé i18n du libellé, grand, multiligne. */
    field: Snippet<[string, string, boolean, boolean]>;
    /** Ligne(s) piste / disque, propres à chaque éditeur. */
    numbers: Snippet;
  } = $props();
</script>

{#if loading}
  <!-- Le squelette reprend la forme et l'écart des blocs : la mise en page ne
       bouge pas quand les valeurs arrivent. -->
  <div class="space-y-5" aria-hidden="true">
    {#each [2, 4, 2] as rows, group (group)}
      <div class="space-y-2">
        <div class="h-2 w-20 rounded bg-neutral-200/80 dark:bg-white/8 animate-pulse"></div>
        <div class="space-y-1.5">
          {#each Array.from({ length: rows }) as _, i (i)}
            <div class="h-12.5 rounded-lg animate-pulse
                        bg-neutral-100/70 dark:bg-white/4"></div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
{:else}
  <!-- Blocs espacés : c'est le titre et l'écart qui groupent, pas un cadre. -->
  <section>
    <TagGroupTitle label={$t('tags.section_track')} />
    <div class="space-y-1.5">
      {@render field('title', 'tags.title', true, false)}
      {@render field('artist', 'tags.artist', false, false)}
    </div>
  </section>

  <section>
    <TagGroupTitle label={$t('tags.section_album')} />
    <div class="space-y-1.5">
      {@render field('album', 'tags.album', false, false)}
      {@render field('album_artist', 'tags.album_artist', false, false)}
      <!-- L'année est courte : la largeur d'un genre gaspillerait la ligne. -->
      <div class="grid grid-cols-3 gap-1.5">
        {@render field('year', 'tags.year', false, false)}
        <div class="col-span-2">
          {@render field('genre', 'tags.genre', false, false)}
        </div>
      </div>
      {@render numbers()}
    </div>
  </section>

  <section>
    <TagGroupTitle label={$t('tags.section_extra')} />
    <div class="space-y-1.5">
      {@render field('composer', 'tags.composer', false, false)}
      <!-- Multiligne : un commentaire tient rarement sur une ligne. -->
      {@render field('comment', 'tags.comment', false, true)}
    </div>
  </section>
{/if}
