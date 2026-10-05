<script lang="ts">
import { page } from "$app/state";
import ImportAccueil from "#lib/components/library/common/ImportAccueil.svelte";
</script>

<ImportAccueil section={page.url.searchParams.get("section")} />
