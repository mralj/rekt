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

    #[arg(
        short = 'm',
        long = "important",
        default_value = "true",
        value_name = "Is server important?"
    )]
    pub is_important_server: bool,

    #[arg(
        long = "pings_unimportant",
        default_value = "1",
        value_name = "Pings per unimportant server"
    )]
    pub pings_per_unimportant_server: usize,
}

impl Display for Cli {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "server_index: {}, pings_per_server: {}, is_important_server: {}",
            self.server_index, self.pings_per_server, self.is_important_server
        )
    }
}
