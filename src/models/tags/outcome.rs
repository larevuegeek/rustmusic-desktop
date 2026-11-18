#[derive(Debug, Clone)]
pub struct Outcome {
    pub batch_id: String,
    pub succeeded: usize,
    pub failed: Vec<(String, String)>,
}
