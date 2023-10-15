use std::str::FromStr;

use ethers::{
    middleware::MiddlewareBuilder,
    prelude::k256::ecdsa::SigningKey,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, Wallet},
    types::{BlockNumber, U256},
};
use futures::stream::FuturesUnordered;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

const DEFAULT_PUBLIC_NODE_QUERY_TIMEOUT_IN_SEC: u64 = 5;

const PUBLIC_NODE_URLS: [&str; 6] = [
    "https://bscrpc.com",
    "https://bsc-dataseed.binance.org/",
    "https://rpc.ankr.com/bsc",
    "https://bsc-dataseed1.defibit.io/",
    "https://bsc-dataseed1.ninicoin.io/",
    "https://bsc.nodereal.io",
];

static PUBLIC_NODES: Lazy<RwLock<Vec<Provider<Http>>>> = Lazy::new(|| RwLock::new(Vec::new()));

pub async fn init_connection_to_public_nodes() {
    for rpc_url in PUBLIC_NODE_URLS.iter() {
        let mut public_nodes = PUBLIC_NODES.write().await;
        if let Ok(p) = Provider::<Http>::try_from(*rpc_url) {
            match p.get_block_number().await {
                Ok(b_no) => {
                    println!("Connected to public node: {rpc_url}, Highest known block {b_no}");
                    public_nodes.push(p);
                }
                Err(e) => {
                    println!("Failed to connect to public node: {}", e);
                }
            }
        }
    }
}

pub async fn get_nonces() {
    let private_keys = vec![
        "c9aebfba092f657150d66df2ec450e56b3d36cbb6c2f54c517208f003019d075",
        "72f6f94063935787b1a3a10e97fe5300c1d1da237dd0a1b0b83f5676a87f1a41",
        "88565bd29f41084bb57333ec4f458df9647f7a2216643a9670cd6ce5dfdde52b",
        "a4166e95e71f53ad469144eb034aa1beee517cd14513dbb49743fe9ee29839b2",
        "0af54ac661e593d6b3d34d3e0366e0c221651cce8b518cf8424cf9260ce6ace3",
        "bd1bbd7a99228e2cc40e589474d3b7d7393751b0edd669d9af342992394621be",
        "e22b68b87e5b52479dc4c9818ce3840aec79e917ed3fc3ab33639c437d6b4b90",
        "387f73baa0e605b91cba78386c2b303db36b59d5447f1286e1b7689f7f929036",
        "fc3dce9c1b1958f3d6b6944f988c2d2d216468cafa8ae48a4ae17ddc96d06806",
        "55db6611abb294d3adf44155e6884ef146e679162ef44cc4f9585a81f1a83583",
        "24d2a1eadae608aed82ea7d6e70425044f815f1aa299d3145496f0a1d18fb19c",
        "94f136956f1c49810d6bbde642d65c31b45c1278c5b6e647a9a2d628eadea973",
        "2faa8d748a4291da2dc82577a47c07dfe147bac12eb7f9fed1592a1ced2d36b4",
        "9cdc5fb980a60a45da182663d99ebd2a360df3900857b0f208d5e014c55226be",
        "cd2961fe102a3bf218fc3ab554a294fb49fa3849516668172c6ed476553599f4",
        "19e89efd38ef3fb403e72065e8304f859acb1a3255ed79a2ec250931c57064ec",
        "6c92e23ccaa0921b57d5a535110f0f32970f3e721617953c670f581218632adf",
        "551c04718c3a1efd6a26692ea3a65abdefabbaf4d28c420869d6f49fd484f78b",
        "8b4b26d5127b88afab546f4ea9fe4ba3443d934aacfc7c6efc11aa1675c85c9b",
        "7d1904377ef7798c146ba8d52b15c05d0f833e15795879361dd076817ce6989e",
        "b27406c6e5670a86007264506fbe5efdc9a4c22d454eeb4a304b5b34f923c38b",
        "7a47e25ed396926ce9076c2d0320d52608f8eb89ef417f3b37dd8361b7bf63fc",
        "4342d380a583cc12d7edba2e541bfd6731ff58b18a6cbcf7b5c7a4e9473c5b71",
        "50cf97c6bea8bba8141fdda4860b41438f9d35a39449c106c6e7ac45a6e3da93",
        "1eecc693ff28b05f72a7ecb025fcf458ec94d2c351fb867085ecbadabc3da251",
        "d9eb40b9a7a65a112112b7fa20f505ff19a6a08d3f5208f0e19c175920eaa1b9",
        "1340541799ccb6870aae4eb4bf192037eaa38aba4ebda8cace4a9bd4523671f4",
        "d0e265cf72d0f3ed129a85de7bdf68444c48fee08d3866c3191280492fcbca31",
        "f69f176f1dc88af45641a073e9b470b8805188319381881cc8ea36290aad30c6",
        "370cc3f32afe5dc030b7b5c1f443a7925d5f245eca1866a586df3260a672f00b",
        "7b4fc1f2c60f92311ab33459fa47642cb7d366925c36ba359962c2386537e38d",
        "a8e45939fd3dd4eb4d241dfc80748cfdfe402c2f7dd8e7766414772f9ddf21c2",
        "35e0da43f10f7535a0a3917910af31e3d64d670e2075bf160d8ea9055cceac3f",
        "29e5f6a76b01caa2dc95269d7edad9c0b710b2e0395396247755039a2fc6586b",
        "0e37851ea874cac4d5464c4944b7b58315d8517c5c9a733769ed56c12476ef29",
        "20ce261c46746fe18247bf8e3dea91462d72c83cd599b2a9b18db8145bf18033",
        "c58693906bd6bedb5c6b02fdab6e1e0eede00ce48bb990ea5f1e58f544c5975f",
        "cb871b4f81664b5e25df7e063ef94490130a374093221284cb9fe41159e10673",
        "b34595d21161672eac30a616a3cec67860153d0dbf8aada18be810b2274a30f1",
        "0601d8865f6f64672b33a2092d81277af620f9e6b97a3c5d00076fe301c897f8",
        "07095b1343ecd817a5c9f3c4ee0cd74aceac10c17acb41437b982dfa065d6208",
        "fb4a2555373978eacc2ab9b47210c03747fb81a9e59dc88fbc8c9a01b3a9f675",
        "4dee7822548c5109550b4583427a08363797783c74eea7aed00c2eede800857c",
        "164371d610747cc95f65fe396860d2dfff37a4cca82494c6a7d161c7eeeb2eb8",
        "36e451544976800cd48f0b1adf5f7cceb8b9395fa591fe8b758da5e1a466eb62",
        "a3521368ceca212f9d0e75d50e6108d76c3cb15c7df642c1b45c779e47b35603",
        "ce1ce7fa1607d620d0368f6af2859c012a324aeb0f472b81281f73012f3d17d7",
        "cf5283aba11c501b49ebce8759efd74c9780f1ed11961c4e6a4816ae2b923df4",
        "65f1e04cef32a1b872948724a3c79aa26b745cc73236c9f418335b0087addf51",
        "b4baa7c9f594778988e9cf6a22c447a50bbd0c510dd726eb23cfd2b666019621",
        "9d1d09e4ede614433e889ed17ae658043ffc9407d82d39d7d37891b428605a28",
        "39c58b8d61721d544c94583ad27611f48e1b9d6c869694d76f3343eb5f68f165",
        "139d21e094a14afda53367a41b9b745562df3e641ac1f4939adac8e4a25ec9f8",
        "f74a7dda0f7c050cacf10e9a6f7f347bee5715c7c61fd413b987f72e0aa749d9",
        "d2657b9ff0ea449721f0db30a35198520679489c28d5e877b7c1fcb037622641",
        "42a5edec989c61f6be3383f08ca02aff92de6d218a7de52e8760c5979018d05b",
        "266d67e533f5b284b2a05c769bd625e4414c6b1e3c62b641dad6d3bce328c517",
        "2346d0d3e3c061530f2907b4257ce0ab62c20c44d8641c9bf44369493bb2c8d7",
        "03313fa2f349d72bea9c6f71ba6c5169fbb57b20a85e7965a76208fe4f84c90e",
        "9d1691048d4eab7fd01854da3bc0027c6c0d433063f83243956cb4331c9e057e",
        "526c6f8824e2e1a37f7fd3a5b6f0f9639668f819ac9cac0571ea6c22016ee9ce",
        "dd9acb1fa434c5d7894576246ef9426eecd8a06ceceb85d4e2bfc910b953f094",
        "b58df0a0068004039eeffa21d6eb5305f475b313819d3b4316f26799109f413d",
        "5322149cf133531f45491c1528fc7fafa9dfa5608c3b38734e81cc53173b4f1b",
        "426c8fb97ef69da28f71c904e4e57ab0a42d917a9b4a840176862e21f2592a24",
        "3b68467361343cca1f9f273698f8a6e37810c22174ef11b29795e23c051f0478",
        "81865dbafd25a6119f72ef3ca4967cefa738bd806f89283257a871af484e154c",
        "4df0f2d2681d47ce264c315d6bf28f0e4992fe745767e15464d5f455da1e673a",
        "88045ac3510433e3840e25193d3254363cd5bdacf1b270259adffeb874810bdc",
        "d31f18c20515b3217b4f9af6a0d2b8b2579bc5fe96431849fdedaedcd240398b",
        "d4bedb2d06302d0607a038ef1d3e5447fa3e7f6ce1a3dc9bf578552f76217595",
        "787ef6e71d7c354861a227eeee2bdd1ac0a6589dcf20f5390c45c5974352941d",
        "3a0f0b02ba02cc30a1963d65f1fb9eaa8835d8030cedc9348b5c2bf6ca38a796",
        "2846e8f8c3178fcba639dc4987c4f95ebe4022ce884185881426f8728ddf393e",
        "e9d830d309b49dd39e050155aaf78694b161826830d9ff2593f25cca0254c2ca",
        "9379f12299b24b1983232923dfe3bef20bd54cd9389c488c3a987c380cd588d3",
        "6d4ab24c1dda124d604fac8df89233b2fe587b6569266864c441abb31ba54e2a",
        "e79d8dd4b15f0b3b0f0acbd8a180e7966e23b14d7cbf09fd7c45abf7e1aad943",
        "d895388e55c90cd227793aa528b8f236ddb65cb249e9f04d0aa8af87112f3202",
        "208fa9351c9128a2399e491a2e62003b0c0676f6f52cddbf1bf94064bd937e90",
        "89a09b661d79c2c064d881827af7fddf7906491231a6ed75ad7c95e74fff224c",
        "6ec8046fb65a15d7f1ac57594c6faf3c0d7fdf360b69e871e618140016f0f52c",
        "23e2dd03bd224ba7af6cf504b8a9186e00593d481da5ac6ea1e2a92be0b82bd7",
        "dd7af3f86cdf3a3831ca4d1b396907ff1719dd450968e353738a1f395c9ba251",
        "371e453a54245a5fd7831e522388f2a71260dcd730bf0b1a906a6628e3f5febe",
        "3452bd6790cc764f83fa1148850c34dfd6d194e100e397d2eace0c12abecd64c",
        "4a3a9781e7ea31d94bfd883b3f950a59d62e9d461c71210651311eed68625e3b",
        "15f971f9b623d2bf2e3c06edddeea56ec18c1718cb59cdb40ea11ad188f92cf4",
        "7f6033da46d1027903ee321dea37ba3f52fba364a92bc746ee84d56f26476395",
        "9771217dac4772e4591ac68d5ac3998e1594cf6c96a9bee5b8c0382917caa8a1",
        "a06a24cbd0153bb08d4a09cf0fbc93fa3cd7d307793bafc21f1e1ea3c3ae6bff",
        "673363657e37b153a07a2263f44d93b5552382ebbb5a5bf28b97c91b50fa2bd2",
        "0da6761e60f3e062292bebeb3be2b3ebcfcf7af33636905cac238e76e4a3abbf",
        "e804bdec0534dd41fc408e200da35c51f8485728137e6bf46caf793b7327675c",
        "a5f7cb6011e8e4d3ad7c5d8d872830e45da04e35c9ecd6926699bce1d4cf02d7",
        "847c8b0d7b83bc0258b1c69824494759bbc8883ba98c7f3c9eea749308bc8f6d",
        "458a4ecea6df4d7ee32584a9c09a5dac6b4d15bc0e9206601edf4811e7e72917",
        "3c9003f7296741b78eddf6d2b16f227fa8696aa21100d979f29844868da88be5",
        "af8551e9c7ff900d4386014ff4b746cc5cbb91d3308b154ddba639acf9c8a0db",
    ];

    let wallets: Vec<Wallet<SigningKey>> = private_keys
        .iter()
        .filter_map(|k| LocalWallet::from_str(k).ok())
        .collect();

    if wallets.len() != private_keys.len() {
        panic!("Failed to parse all private keys");
    }

    let mut nonce_tasks = FuturesUnordered::from_iter(wallets.iter().map(|w| get_nonce(w)));
    let start = std::time::Instant::now();
    while let Some(nonce) = nonce_tasks.next().await {
        let _n = nonce;
    }
    println!(
        "Took {:?} to get nonces for {} wallets",
        start.elapsed(),
        wallets.len()
    );
}

pub async fn get_nonce(wallet: &Wallet<SigningKey>) -> U256 {
    let providers = PUBLIC_NODES.read().await;
    let mut nonce_tasks = FuturesUnordered::from_iter(providers.iter().map(|p| {
        tokio::time::timeout(
            std::time::Duration::from_secs(DEFAULT_PUBLIC_NODE_QUERY_TIMEOUT_IN_SEC),
            p.get_transaction_count(wallet.address(), Some(BlockNumber::Pending.into())),
        )
    }));

    if let Some(Ok(Ok(nonce))) = nonce_tasks.next().await {
        println!("Nonce for address {} is {nonce}", wallet.address());
        return nonce;
    }

    return U256::zero();
}
