use crate::models::servers::*;
use crate::models::servers::DlnaPick;
use crate::core::dlna_client::client::Found;

#[derive(Debug, Clone)]
pub struct AddServer {
    pub kind: usize,
    pub url: String,
    pub username: String,
    pub password: String,
    pub api_key: String,
    pub with_key: bool,
    pub probe: Option<Probe>,
    pub name: String,
    pub chosen: Vec<String>,
    pub dlna: Option<DlnaPick>,
    pub found: Vec<Found>,
    pub searching: bool,
    pub busy: bool,
    pub error: Option<String>,
}
