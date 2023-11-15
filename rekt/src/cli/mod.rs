use std::fmt::{Display, Formatter};

use clap::Parser;

#[derive(Parser, Debug, Clone)]
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
        short = 'u',
        long = "important",
        value_name = "Is server unimportant important?"
    )]
    pub is_un_important_server: bool,

    #[arg(
        long = "pings_unimportant",
        default_value = "1",
        value_name = "Pings per unimportant server"
    )]
    pub pings_per_unimportant_server: usize,

    #[arg(long = "name", default_value = "N/A", value_name = "Server name")]
    pub name: String,

    #[arg(long = "country", default_value = "N/A", value_name = "Server country")]
    pub country: String,

    #[arg(long = "city", default_value = "N/A", value_name = "Server country")]
    pub city: String,

    pub first_wallet: Option<ethers::types::Address>,
    pub last_wallet: Option<ethers::types::Address>,
}

impl Cli {
    pub fn set_first_last_wallets(
        &mut self,
        first_wallet: ethers::types::Address,
        last_wallet: ethers::types::Address,
    ) {
        self.first_wallet = Some(first_wallet);
        self.last_wallet = Some(last_wallet);
    }
}

impl Display for Cli {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "server_index: {}, pings_per_server: {}, is_un_important_server: {},name: {}, county: {}, city: {}\n first_wallet: {:?}, last_wallet: {:?}",
            self.server_index, self.pings_per_server, self.is_un_important_server, self.name, self.country, self.city, self.first_wallet, self.last_wallet
        )
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            server_index: 1,
            pings_per_server: 10,
            is_un_important_server: false,
            pings_per_unimportant_server: 1,
            name: "N/A".into(),
            country: "N/A".into(),
            city: "N/A".into(),
            first_wallet: None,
            last_wallet: None,
        }
    }
}
