<script lang="ts">
  // HTML venu de l'extérieur (description de podcast) : nettoyé, puis mis en forme sobrement.
  import { safeHtml } from "#lib/helper/tools/safeHtml";

  let { html, class: classes = "" }: { html: string | null | undefined; class?: string } = $props();

  const propre = $derived(safeHtml(html));
</script>

<div class="text-pretty break-words [&_p]:mb-2 [&_p:last-child]:mb-0 [&_a]:text-(--rg-gtx) [&_a]:underline [&_a]:underline-offset-2
            [&_ul]:list-disc [&_ul]:pl-5 [&_ul]:mb-2 [&_ol]:list-decimal [&_ol]:pl-5 [&_ol]:mb-2 [&_li]:mb-0.5
            [&_blockquote]:border-l-2 [&_blockquote]:border-(--rg-bd2) [&_blockquote]:pl-3 [&_strong]:font-semibold [&_b]:font-semibold {classes}">
  {@html propre}
</div>
