<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";

  /**
   * Notation en demi-étoiles.
   *
   * `value` est un nombre d'étoiles, de 0,5 à 5,0 par pas d'un demi : 3.5 se
   * lit « trois étoiles et demie ». C'est l'unité stockée en base, et la garder
   * telle quelle jusqu'ici évite un aller-retour de conversion à chaque
   * affichage — et les erreurs d'arrondi qui viennent avec.
   *
   * Le flottant ne pose pas de problème d'exactitude ici : les demis ont un
   * dénominateur en puissance de deux, donc 0,5 · 1,0 · 1,5 … 5,0 sont toutes
   * représentables au bit près.
   *
   * `null` signifie « jamais noté », ce qui n'est pas la même chose que zéro :
   * les tris rangent les non-notés à part.
   */
  let {
    trackId,
    value = null,
    size = 12,
    readonly = false,
    onchange,
  }: {
    trackId?: string;
    value?: number | null;
    size?: number;
    readonly?: boolean;
    onchange?: (rating: number | null) => void;
  } = $props();

  let internalValue = $state<number | null>(null);
  let hoverValue = $state<number | null>(null);
  let current = $derived(hoverValue ?? internalValue ?? 0);

  $effect(() => { internalValue = value ?? null; });

  /** 0 = vide, 1 = moitié, 2 = pleine, pour la n-ième étoile (1 à 5). */
  function fillOf(star: number): 0 | 1 | 2 {
    if (current >= star) return 2;
    if (current >= star - 0.5) return 1;
    return 0;
  }

  async function handleClick(stars: number) {
    if (readonly || !trackId) return;

    // Recliquer sur le cran courant efface la note : c'est le seul geste
    // disponible pour dénoter, la valeur zéro n'étant pas atteignable.
    const newRating = internalValue === stars ? null : stars;
    const previous = internalValue;
    internalValue = newRating;
    try {
      await invoke('set_track_rating', { trackId, rating: newRating });
      onchange?.(newRating);
    } catch (e) {
      internalValue = previous;
      console.error('Failed to set rating:', e);
    }
  }

  function label(stars: number): string {
    const texte = Number.isInteger(stars)
      ? String(stars)
      : `${Math.floor(stars)} et demie`;
    return `${texte} étoile${stars > 1 ? 's' : ''}`;
  }
</script>

<div
  class="flex items-center gap-0.5"
  onmouseleave={() => hoverValue = null}
  role="presentation"
>
  {#each [1, 2, 3, 4, 5] as star}
    {@const fill = fillOf(star)}
    <div
      class="relative shrink-0 transition-transform duration-100
             {readonly ? '' : 'hover:scale-110'}"
      style="width:{size}px;height:{size}px"
    >
      <!-- Fond : l'étoile vide, toujours présente, sert de contour. -->
      <Icon
        icon="lucide:star"
        width={size}
        class="absolute inset-0 text-neutral-300 dark:text-neutral-600"
      />

      <!-- Remplissage : l'étoile pleine posée par-dessus, rognée à mi-largeur
           pour le demi-cran. Découper le calque plutôt que d'employer une
           icône « demi-étoile » dédiée garantit que les deux moitiés
           coïncident exactement, quelle que soit la taille demandée. -->
      {#if fill > 0}
        <span
          class="absolute inset-y-0 left-0 overflow-hidden pointer-events-none"
          style="width:{fill === 1 ? size / 2 : size}px"
        >
          <Icon
            icon="mynaui:star-solid"
            width={size}
            class="text-green-500 drop-shadow-[0_0_4px_rgba(34,197,94,0.35)]"
          />
        </span>
      {/if}

      <!-- Deux zones de clic superposées à l'étoile : la gauche pose le demi
           cran, la droite l'étoile pleine. Deux boutons distincts plutôt qu'un
           calcul de position du curseur — le clavier et les lecteurs d'écran
           les atteignent, et rien ne dépend de la géométrie rendue. -->
      {#if !readonly}
        <button
          type="button"
          class="absolute inset-y-0 left-0 w-1/2 cursor-pointer"
          onmouseenter={() => hoverValue = star - 0.5}
          onclick={(e) => { e.stopPropagation(); handleClick(star - 0.5); }}
          aria-label={label(star - 0.5)}
        ></button>
        <button
          type="button"
          class="absolute inset-y-0 right-0 w-1/2 cursor-pointer"
          onmouseenter={() => hoverValue = star}
          onclick={(e) => { e.stopPropagation(); handleClick(star); }}
          aria-label={label(star)}
        ></button>
      {/if}
    </div>
  {/each}
</div>
