<script lang="ts">
import { resolveCoverSrc, type CoverSize } from "$lib/helper/tools/coverHelper";
import FadeImg from "./FadeImg.svelte";
import type { HTMLImgAttributes } from "svelte/elements";

let { path, alt = "", size = 'full', ...rest }: {
    path: string | null | undefined;
    alt?: string;
    size?: CoverSize;
} & Omit<HTMLImgAttributes, "src" | "alt"> = $props();

let src = $state<string | null>(null);

$effect(() => {
    const currentPath = path;
    if (!currentPath) {
        src = null;
        return;
    }
    // Pochette changée avant la réponse : l'ancienne ne doit pas s'afficher.
    let vivant = true;
    resolveCoverSrc(currentPath, size).then(url => {
        if (vivant) src = url;
    });
    return () => { vivant = false; };
});
</script>

{#if src}
    <FadeImg {src} {alt} {...rest} />
{/if}
