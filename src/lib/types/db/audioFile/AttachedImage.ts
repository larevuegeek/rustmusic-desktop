export type ImageType = 
    | "Other"
    | "Icon32x32PNG"
    | "IconOther"
    | "CoverFront"
    | "CoverBack"
    | "LeafletPage"
    | "MediaLabel"
    | "LeadArtist"
    | "Artist"
    | "Conductor"
    | "BandOrchestra"
    | "Composer"
    | "LyricistTextWriter"
    | "RecordingLocation"
    | "DuringRecording"
    | "DuringPerformance"
    | "MovieVideoScreenCapture"
    | "ABrightColouredFish"
    | "Illustration"
    | "BandArtistLogo"
    | "PublisherStudioLogo";

export interface AttachedImage {
    image_type?: ImageType; // Supposons que c'est optionnel dans TypeScript
    mime_type: string; // MIME type de l'image, par exemple, "image/jpeg"
    description?: string; // Description optionnelle de l'image
    /** Jamais transmis par le backend — voir `AttachedImage` côté Rust. */
    image_data?: never;
    image_src: string; // Source de l'image, pourrait être une URL ou un chemin local
}