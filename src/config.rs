/*
All config related stuff is here.

*/
use serde::{Deserialize, Serialize};
use std::{fs::File, io::{BufReader, Write}, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertMode {
    SelfSigned,
    Manual,
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemConfig {
    pub http: u16,
    pub https: u16,
    pub cert_mode: CertMode,
}

// Load in or create the YAML if it doesn't exist already.
pub fn load_or_create_config(path: &str) -> SystemConfig {
    if !Path::new(path).exists() {
        let default = SystemConfig {
            http: 8080,
            https: 8443,
            cert_mode: CertMode::None,
        };

        let yaml = serde_yaml::to_string(&default).expect("Failed to serialize default config");
        let mut file = File::create(path).expect("Failed to create config.yaml");
        file.write_all(yaml.as_bytes()).expect("Failed to write default config");
    }

    let file = File::open(path).expect("Failed to open config.yaml");
    let reader = BufReader::new(file);
    serde_yaml::from_reader(reader).expect("Failed to parse YAML config")
}
