<script lang="ts">
import { page } from "$app/state";
import ImportWelcome from "#lib/components/library/common/ImportWelcome.svelte";
</script>

<ImportWelcome section={page.url.searchParams.get("section")} />
