use std::fmt::{Display, Formatter};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(
        short = 'i',
        long = "index",
        default_value = "1",
        value_name = "Server index"
    )]
    pub server_index: usize,
    #[arg(
        short = 'p',
        long = "pings",
        default_value = "1",
        value_name = "Pings per server"
    )]
    pub pings_per_server: usize,
}

impl Display for Cli {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "server_index: {}, pings_per_server: {}",
            self.server_index, self.pings_per_server
        )
    }
}
