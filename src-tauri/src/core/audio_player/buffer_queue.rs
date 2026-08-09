//! File de tampons audio décodés — un tampon par piste.
//!
//! # Pourquoi une file plutôt qu'un tampon unique
//! Historiquement la voie Symphonia décode dans **un seul** `Vec<f32>` qui
//! grandit, lu par le backend de sortie via un curseur. Ça marche pour une
//! piste isolée, mais pas pour l'enchaînement sans blanc :
//!
//! - concaténer les pistes ferait croître le `Vec` sans limite (un album
//!   entier en RAM), et le compacter obligerait à décaler des mégaoctets sous
//!   verrou pendant que le callback lit ;
//! - il faudrait tenir une comptabilité de décalages pour savoir quelle piste
//!   joue et à quelle position ;
//! - annuler une piste déjà décodée serait impossible sans toucher à ce qui
//!   est en train de jouer.
//!
//! Une file d'entrées indépendantes résout les trois : on **retire** l'entrée
//! consommée (mémoire bornée à ce qui est en vol), chaque entrée porte son
//! identité et sa position (aucun décalage à calculer), et annuler la piste
//! suivante revient à retirer une entrée que **personne n'a encore lue** —
//! opération gratuite et silencieuse.
//!
//! # Modèle de concurrence
//! Une seule structure protégée par un `Mutex`. Le décodeur y ajoute
//! (`append`), le backend de sortie y lit (`read`) depuis son callback temps
//! réel, la boucle de supervision l'interroge. Les sections critiques sont
//! courtes et sans allocation côté lecture : `read` ne fait que des `copy` et
//! un `pop_front` occasionnel.
//!
//! Le callback audio ne doit **jamais** émettre d'évènement Tauri (thread
//! temps réel) : il se contente d'enregistrer le franchissement de frontière
//! dans [`ReadOutcome`], et c'est la boucle de supervision qui le publie.

use std::collections::VecDeque;

/// Un morceau décodé (ou en cours de décodage) en attente de lecture.
#[derive(Debug)]
struct TrackBuffer {
    /// Identité unique de cette occurrence dans la file. Deux lectures
    /// successives du même fichier (repeat one) ont deux jetons distincts.
    token: u64,
    /// Échantillons f32 interleavés, **au format de sortie** (rate et canaux
    /// déjà adaptés par le décodeur).
    samples: Vec<f32>,
    /// `true` quand le décodeur a fini de remplir cette entrée.
    complete: bool,
    /// Facteur Replay Gain propre à CETTE piste, appliqué à la lecture.
    ///
    /// C'est ce qui rend le gain correct en enchaînement : appliqué au
    /// moment où les échantillons partent vers le DAC, donc au bon instant,
    /// et non au décodage (qui a une à deux secondes d'avance).
    gain: f32,
}

/// Ce que la lecture a produit, pour la boucle de supervision.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ReadOutcome {
    /// Nombre d'échantillons réellement écrits dans la sortie.
    pub samples_written: usize,
    /// Jeton de la piste qui vient de se terminer, si une frontière a été
    /// franchie pendant cette lecture. La boucle de supervision s'en sert
    /// pour émettre `track-advanced`.
    pub finished_token: Option<u64>,
    /// `true` si la file est à sec alors que la piste courante n'est pas
    /// terminée (sous-alimentation du décodeur).
    pub starved: bool,
}

/// File de tampons décodés, une entrée par piste.
#[derive(Debug)]
pub struct BufferQueue {
    entries: VecDeque<TrackBuffer>,
    /// Curseur de lecture **en échantillons** dans `entries[0]`.
    cursor: usize,
    /// Canaux de sortie — sert à convertir échantillons ↔ frames.
    output_channels: u16,
}

// Certaines méthodes ne servent qu'à l'enchaînement des pistes (étape
// suivante) : elles sont testées mais pas encore appelées par le lecteur.
#[allow(dead_code)]
impl BufferQueue {
    pub fn new(output_channels: u16) -> Self {
        Self {
            entries: VecDeque::new(),
            cursor: 0,
            output_channels: output_channels.max(1),
        }
    }

    /// Ajoute une piste en fin de file (la piste courante n'est pas touchée).
    pub fn push_track(&mut self, token: u64, gain: f32) {
        self.entries.push_back(TrackBuffer {
            token,
            samples: Vec::new(),
            complete: false,
            gain,
        });
    }

    /// Ajoute des échantillons décodés à la piste `token`.
    ///
    /// Sans effet si le jeton n'est plus dans la file : c'est le cas normal
    /// quand un décodeur en vol a été invalidé (changement de file d'attente)
    /// et n'a pas encore vu la demande d'arrêt.
    pub fn append(&mut self, token: u64, samples: &[f32]) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.token == token) {
            entry.samples.extend_from_slice(samples);
        }
    }

    /// Marque une piste comme entièrement décodée.
    pub fn mark_complete(&mut self, token: u64) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.token == token) {
            entry.complete = true;
        }
    }

    /// Remplace le facteur Replay Gain d'une piste encore en file.
    pub fn set_gain(&mut self, token: u64, gain: f32) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.token == token) {
            entry.gain = gain;
        }
    }

    /// Remplit `out` depuis la file, en traversant les frontières de piste.
    ///
    /// Appelé depuis le callback audio : aucune allocation, aucune émission
    /// d'évènement. Le reste (silence, volume global, fade-in de seek) reste
    /// à la charge du backend appelant.
    pub fn read(&mut self, out: &mut [f32]) -> ReadOutcome {
        let mut outcome = ReadOutcome::default();
        let mut written = 0usize;

        while written < out.len() {
            let Some(front) = self.entries.front() else {
                break; // Plus rien en file.
            };

            let available = front.samples.len().saturating_sub(self.cursor);

            if available == 0 {
                if front.complete {
                    // Piste terminée ET entièrement lue → on la retire. La
                    // mémoire est libérée ici, ce qui borne l'empreinte à ce
                    // qui reste à jouer.
                    let done = self.entries.pop_front();
                    self.cursor = 0;
                    if let Some(done) = done {
                        // On ne signale qu'une frontière par appel : si une
                        // piste entière tenait dans un seul callback, la
                        // suivante sera signalée au tour d'après.
                        if outcome.finished_token.is_none() {
                            outcome.finished_token = Some(done.token);
                        } else {
                            self.entries.push_front(done);
                            break;
                        }
                    }
                    continue;
                }
                // Décodeur en retard : on s'arrête là, l'appelant complétera
                // avec du silence.
                outcome.starved = true;
                break;
            }

            let take = available.min(out.len() - written);
            let gain = front.gain;
            let src = &front.samples[self.cursor..self.cursor + take];

            if (gain - 1.0).abs() < f32::EPSILON {
                out[written..written + take].copy_from_slice(src);
            } else {
                for (dst, s) in out[written..written + take].iter_mut().zip(src) {
                    *dst = *s * gain;
                }
            }

            self.cursor += take;
            written += take;
        }

        outcome.samples_written = written;
        outcome
    }

    /// Jeton de la piste en cours de lecture.
    pub fn current_token(&self) -> Option<u64> {
        self.entries.front().map(|e| e.token)
    }

    /// Position de lecture dans la piste courante, en frames.
    pub fn position_frames(&self) -> usize {
        self.cursor / self.output_channels as usize
    }

    /// Déplace le curseur dans la piste courante (seek).
    ///
    /// Retourne `false` si la cible dépasse ce qui est décodé : l'appelant
    /// doit alors relancer un décodage à partir de la position voulue.
    pub fn seek_current(&mut self, frames: usize) -> bool {
        let target = frames * self.output_channels as usize;
        match self.entries.front() {
            Some(front) if target <= front.samples.len() => {
                self.cursor = target;
                true
            }
            _ => false,
        }
    }

    /// Nombre de frames décodées d'avance sur la piste courante.
    /// Sert à décider quand lancer le décodage de la suivante.
    pub fn buffered_frames(&self) -> usize {
        self.entries
            .front()
            .map(|e| e.samples.len().saturating_sub(self.cursor))
            .unwrap_or(0)
            / self.output_channels as usize
    }

    /// Vrai si une piste est déjà en attente derrière la piste courante.
    pub fn has_pending(&self) -> bool {
        self.entries.len() > 1
    }

    /// Retire toutes les pistes en attente **derrière** la piste courante.
    ///
    /// C'est l'annulation de préchargement : la file d'attente a changé, la
    /// suite décodée n'est plus la bonne. Rien de ce qui joue n'est touché,
    /// donc l'opération est inaudible.
    ///
    /// Retourne les jetons abandonnés, pour que l'appelant arrête les
    /// décodeurs correspondants.
    pub fn drop_pending(&mut self) -> Vec<u64> {
        let mut dropped = Vec::new();
        while self.entries.len() > 1 {
            if let Some(e) = self.entries.pop_back() {
                dropped.push(e.token);
            }
        }
        dropped
    }

    /// Vide entièrement la file (stop, changement de piste manuel).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }

    /// Empreinte mémoire approximative, pour la journalisation et le
    /// plafonnement du préchargement.
    pub fn total_bytes(&self) -> usize {
        self.entries
            .iter()
            .map(|e| e.samples.len() * std::mem::size_of::<f32>())
            .sum()
    }

    /// Vrai si la piste courante est terminée et entièrement lue — donc si la
    /// lecture s'arrête là, faute de suite.
    pub fn is_drained(&self) -> bool {
        match self.entries.front() {
            None => true,
            Some(front) => front.complete && self.cursor >= front.samples.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construit une file stéréo avec une piste complète de `frames` frames,
    /// dont chaque échantillon vaut `value`.
    fn queue_with(token: u64, frames: usize, value: f32) -> BufferQueue {
        let mut q = BufferQueue::new(2);
        q.push_track(token, 1.0);
        q.append(token, &vec![value; frames * 2]);
        q.mark_complete(token);
        q
    }

    #[test]
    fn reads_within_a_single_track() {
        let mut q = queue_with(1, 4, 0.5);
        let mut out = [0.0f32; 4];
        let r = q.read(&mut out);
        assert_eq!(r.samples_written, 4);
        assert_eq!(r.finished_token, None);
        assert_eq!(out, [0.5; 4]);
        assert_eq!(q.position_frames(), 2);
    }

    #[test]
    fn crosses_the_boundary_without_a_hole() {
        let mut q = queue_with(1, 2, 0.25); // 4 échantillons
        q.push_track(2, 1.0);
        q.append(2, &[0.75; 6]);
        q.mark_complete(2);

        let mut out = [0.0f32; 10];
        let r = q.read(&mut out);

        // Tout est servi d'affilée : aucun silence entre les deux pistes.
        assert_eq!(r.samples_written, 10);
        assert_eq!(r.finished_token, Some(1));
        assert_eq!(out[..4], [0.25; 4]);
        assert_eq!(out[4..], [0.75; 6]);
        assert_eq!(q.current_token(), Some(2));
    }

    #[test]
    fn frees_memory_once_a_track_is_consumed() {
        let mut q = queue_with(1, 1000, 0.1);
        q.push_track(2, 1.0);
        q.append(2, &[0.2; 8]);
        q.mark_complete(2);
        let before = q.total_bytes();

        // Première lecture : consomme exactement la piste 1. L'entrée reste en
        // tête (elle demeure disponible pour un seek tant qu'on ne l'a pas
        // dépassée) — la mémoire n'est donc pas encore rendue.
        let mut out = [0.0f32; 2000];
        let first = q.read(&mut out);
        assert_eq!(first.samples_written, 2000);
        assert_eq!(q.current_token(), Some(1));

        // Lecture suivante : la file a besoin de données, retire l'entrée
        // épuisée, signale la frontière et libère la mémoire.
        let second = q.read(&mut out);
        assert_eq!(second.finished_token, Some(1));
        assert_eq!(q.current_token(), Some(2));
        assert!(
            q.total_bytes() < before,
            "l'entrée consommée doit être libérée"
        );
    }

    #[test]
    fn applies_a_gain_per_track() {
        let mut q = BufferQueue::new(2);
        q.push_track(1, 0.5);
        q.append(1, &[1.0; 4]);
        q.mark_complete(1);
        q.push_track(2, 2.0);
        q.append(2, &[1.0; 4]);
        q.mark_complete(2);

        let mut out = [0.0f32; 8];
        q.read(&mut out);

        // Chaque piste porte son propre facteur, appliqué à la frontière exacte.
        assert_eq!(out[..4], [0.5; 4]);
        assert_eq!(out[4..], [2.0; 4]);
    }

    #[test]
    fn signals_starvation_when_the_decoder_is_late() {
        let mut q = BufferQueue::new(2);
        q.push_track(1, 1.0);
        q.append(1, &[0.3; 2]); // incomplète

        let mut out = [0.0f32; 8];
        let r = q.read(&mut out);

        assert_eq!(r.samples_written, 2);
        assert!(r.starved);
        assert_eq!(r.finished_token, None);
    }

    #[test]
    fn dropping_pending_leaves_the_current_track_untouched() {
        let mut q = queue_with(1, 4, 0.4);
        q.push_track(2, 1.0);
        q.append(2, &[0.9; 4]);
        q.push_track(3, 1.0);

        let dropped = q.drop_pending();

        assert_eq!(dropped, vec![3, 2]);
        assert_eq!(q.current_token(), Some(1));
        assert!(!q.has_pending());

        // La piste en cours joue toujours normalement.
        let mut out = [0.0f32; 4];
        let r = q.read(&mut out);
        assert_eq!(r.samples_written, 4);
        assert_eq!(out, [0.4; 4]);
    }

    #[test]
    fn seek_stays_inside_what_is_decoded() {
        let mut q = queue_with(1, 10, 0.6);
        assert!(q.seek_current(5));
        assert_eq!(q.position_frames(), 5);
        // Au-delà du décodé : l'appelant devra relancer un décodage.
        assert!(!q.seek_current(50));
        assert_eq!(q.position_frames(), 5);
    }

    #[test]
    fn reports_drained_only_when_nothing_remains() {
        let mut q = queue_with(1, 2, 0.5);
        assert!(!q.is_drained());
        let mut out = [0.0f32; 4];
        q.read(&mut out);
        assert!(q.is_drained());
    }

    #[test]
    fn one_boundary_per_read_at_most() {
        // Deux pistes minuscules tenant dans un seul callback : la seconde
        // frontière est reportée au tour suivant plutôt que perdue.
        let mut q = queue_with(1, 1, 0.1);
        q.push_track(2, 1.0);
        q.append(2, &[0.2; 2]);
        q.mark_complete(2);

        let mut out = [0.0f32; 16];
        let first = q.read(&mut out);
        assert_eq!(first.finished_token, Some(1));

        let second = q.read(&mut out);
        assert_eq!(second.finished_token, Some(2));
    }
}
