/// Les tâches en cours, pour leurs roues.
#[derive(Default)]
pub struct StorageState {
    pub rescanning: bool,
    pub portraits: bool,
    pub covers: bool,
    pub cleaning: bool,
    pub repairing: bool,
}
