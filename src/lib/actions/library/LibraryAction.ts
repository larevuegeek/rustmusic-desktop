import { goto } from "$app/navigation";
import { choisirDossier, choisirFichiers, importerDossier, importerFichiers } from "#lib/services/library/library.service";
import { libraryStore } from "#lib/stores/library/library.store";
import { libraryContentStore } from "#lib/stores/library/libraryContent.store";
import { toasts } from "#lib/stores/ui/toast.store";
import { importProgressStore } from "#lib/stores/library/importProgress.store";
import { get } from "svelte/store";
import { t, currentLocale } from "#lib/i18n";

/** « 1 piste ajoutée » / « n pistes ajoutées », nombre au format de la langue. */
function pistesAjoutees(n: number): string {
    return get(t)(n === 1 ? "notify.tracks_added_one" : "notify.tracks_added_n").replace("{n}", n.toLocaleString(get(currentLocale)));
}

/** `dejaChoisis` : la page d'import choisit avant de créer la bibliothèque. */
export async function handleAddFiles(libraryId: number, redirectToLibrary = false, dejaChoisis?: string[]): Promise<void> {

    try {
        // Choisir d'abord. Naviguer avant, c'était charger toute la bibliothèque
        // derrière une boîte de dialogue qu'on pouvait encore annuler.
        const fichiers = dejaChoisis ?? await choisirFichiers();
        if (fichiers.length === 0) return;

        if (redirectToLibrary) goto(`/library/${libraryId}`);

        const newTracks = await importerFichiers(libraryId, fichiers);

        if (newTracks.length === 0) return;

        await libraryContentStore.load(libraryId);
        await libraryStore.refresh();

        toasts.push({
            type: "success",
            title: get(t)("notify.files_added"),
            message: pistesAjoutees(newTracks.length)
        });

    } catch (e) {
        toasts.push({
            type: "error",
            title: get(t)("notify.error"),
            message: get(t)("notify.add_files_failed")
        });
    } finally {
        libraryStore.setImporting(false);
    }
}

export async function handleAddDirectory(libraryId: number, redirectToLibrary = false, dejaChoisi?: string) {

    try {
        // Choisir d'abord : annuler doit laisser l'écran tel quel.
        const dossier = dejaChoisi ?? await choisirDossier();
        if (!dossier) return;

        if (redirectToLibrary) goto(`/library/${libraryId}`);

        const newTracks = await importerDossier(libraryId, dossier);

        // Arrêté : importProgressStore affiche le bilan partiel.
        if (newTracks.length === 0 || get(importProgressStore).cancelled) return;

        await libraryContentStore.load(libraryId);
        await libraryStore.refresh();

        toasts.push({
            type: "success",
            title: get(t)("notify.files_added"),
            message: pistesAjoutees(newTracks.length)
        });

    } catch (e) {
        toasts.push({
            type: "error",
            title: get(t)("notify.error"),
            message: get(t)(e === 'deja_en_cours' ? "notify.import_busy" : "notify.add_folder_failed")
        });
    } finally {
        libraryStore.setImporting(false);
    }
}
