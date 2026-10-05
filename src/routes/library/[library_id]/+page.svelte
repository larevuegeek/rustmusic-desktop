<script lang="ts">
import { goto } from "$app/navigation";
import { page } from "$app/state";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import { importAffiche } from "$lib/stores/library/importProgress.store";
import { onMount } from "svelte";
import { dernierOnglet } from "$lib/config/libraryTabs";

const libraryId = $derived(Number(page.params.library_id));

// Simple étape : on file vers le dernier onglet visité.
onMount(() => goto(`/library/${libraryId}/${dernierOnglet(libraryId)}`, { replace: true }));
</script>

{#if $importAffiche}
  <LibraryImportingLoader />
{/if}
