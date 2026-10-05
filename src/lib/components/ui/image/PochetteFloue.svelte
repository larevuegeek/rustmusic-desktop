<script lang="ts">
import { resolveBlurredCover, type CoverSize } from "#lib/helper/tools/coverHelper";
import { settingsStore } from "#lib/stores/settings/settings.store";
import CoverImg from "./CoverImg.svelte";
import FadeImg from "./FadeImg.svelte";

// Pré-floutée par le backend : un `blur()` CSS se recalculait à chaque rafraîchissement.
// `flou` : flou de l'image pré-calculée (64 px) ; `rayon` : flou CSS du mode « en direct ».
let { path, saturation = 1, flou = 4, rayon = 40, size = "2x", class: classe = "" }: {
    path: string | null | undefined;
    saturation?: number;
    flou?: number;
    rayon?: number;
    size?: CoverSize;
    class?: string;
} = $props();

const live = $derived($settingsStore.blurred_backgrounds === "live");

let src = $state<string | null>(null);

$effect(() => {
    if (live) return;
    const p = path;
    const s = saturation;
    const f = flou;
    let current = true;
    resolveBlurredCover(p, s, f).then((url) => {
        if (current) src = url;
    });
    return () => { current = false; };
});
</script>

{#if live}
    <CoverImg {path} {size} class={classe} style="filter: blur({rayon}px) saturate({saturation})" />
{:else if src}
    <FadeImg {src} alt="" class={classe} />
{/if}
