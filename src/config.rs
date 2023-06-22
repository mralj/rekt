use std::io;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub nodes: Vec<String>,
}

pub fn get_config() -> Result<Config, io::Error> {
    let f = get_config_file()?;
    let config: Config = toml::from_str(&f)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Could not parse toml file"))?;

    Ok(config)
}

fn get_config_file() -> Result<String, io::Error> {
    let file_str = std::fs::read_to_string("config.toml")?;
    Ok(file_str)
}
