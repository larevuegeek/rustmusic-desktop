/// Ce que la sortie accepte : fréquences et profondeurs exclusives, format du mixeur.
#[derive(Debug, Clone, Default)]
pub struct OutputCaps {
    pub rates: Vec<u32>,
    pub bits: Vec<u16>,
    pub mix_rate: Option<u32>,
    pub mix_bits: Option<u16>,
    pub mix_channels: Option<u16>,
}
