import { writable } from "svelte/store";

/** Vitesse de lecture en vigueur : celle de l'émission pendant un épisode, 1 sinon. */
export const playbackSpeed = writable(1);

/** Vitesses proposées (le moteur accepte de 0,5 à 3). */
export const SPEEDS = [0.75, 1, 1.25, 1.5, 1.75, 2];
