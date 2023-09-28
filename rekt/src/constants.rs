pub const KB: usize = 1024;
pub const DEFAULT_PORT: u16 = 30311;

pub const BOOTSTRAP_NODES: [&str; 23] = [
    "enode://1cc4534b14cfe351ab740a1418ab944a234ca2f702915eadb7e558a02010cb7c5a8c295a3b56bcefa7701c07752acd5539cb13df2aab8ae2d98934d712611443@52.71.43.172:30311",
	"enode://28b1d16562dac280dacaaf45d54516b85bc6c994252a9825c5cc4e080d3e53446d05f63ba495ea7d44d6c316b54cd92b245c5c328c37da24605c4a93a0d099c4@34.246.65.14:30311",
	"enode://5a7b996048d1b0a07683a949662c87c09b55247ce774aeee10bb886892e586e3c604564393292e38ef43c023ee9981e1f8b335766ec4f0f256e57f8640b079d5@35.73.137.11:30311",
    "enode://433c8bfdf53a3e2268ccb1b829e47f629793291cbddf0c76ae626da802f90532251fc558e2e0d10d6725e759088439bf1cd4714716b03a259a35d4b2e4acfa7f@52.69.102.73:30311",
	"enode://571bee8fb902a625942f10a770ccf727ae2ba1bab2a2b64e121594a99c9437317f6166a395670a00b7d93647eacafe598b6bbcef15b40b6d1a10243865a3e80f@35.73.84.120:30311",
	"enode://fac42fb0ba082b7d1eebded216db42161163d42e4f52c9e47716946d64468a62da4ba0b1cac0df5e8bf1e5284861d757339751c33d51dfef318be5168803d0b5@18.203.152.54:30311",
	"enode://3063d1c9e1b824cfbb7c7b6abafa34faec6bb4e7e06941d218d760acdd7963b274278c5c3e63914bd6d1b58504c59ec5522c56f883baceb8538674b92da48a96@34.250.32.100:30311",
	"enode://ad78c64a4ade83692488aa42e4c94084516e555d3f340d9802c2bf106a3df8868bc46eae083d2de4018f40e8d9a9952c32a0943cd68855a9bc9fd07aac982a6d@34.204.214.24:30311",
	"enode://5db798deb67df75d073f8e2953dad283148133acb520625ea804c9c4ad09a35f13592a762d8f89056248f3889f6dcc33490c145774ea4ff2966982294909b37a@107.20.191.97:30311",
    "enode://fe0bb07eae29e8cfaa5bb15b0db8c386a45b7da2c94e1dabd7ca58b6327eee0c27bdcea4f08db19ea07b9a1391e5496a28c675c6eee578154edae4fa44640c5d@54.228.2.74:30311",
    "enode://c307b4cddec0aea2188eafddedb0a076b9289402c63217b4c81eb7f34761c7cfaf6b075e93d7357169e226ff1bb4aa3bd71869b4c76cf261e2991005ddb4d4aa@3.81.81.182:30311",
    "enode://84a76ad1fab6164cbb00179dd07c96755141ffb75d5d387f45295e6ecfcc9e12a720f1f3dca8318449eeff768d13e9d49a414d2b522d1bcf2919aebf4852ab46@44.198.58.179:30311",
    "enode://41d57b0f00d83016e1bb4eccff0f3034aa49345301b7be96c6bb23a0a852b9b87b9ed11827c188ad409019fb0e578917d722f318665f198340b8a15ae8beff36@34.252.87.229:30311",
    "enode://accbc0a5af0af03e1ec3b5e80544bdceea48011a6928cd82d2c1a9c38b65fd48ec970ba17bd8c0b0ec21a28faec9efe1d1ce55134784b9207146e2f62d8932ba@54.162.32.1:30311",
    "enode://e333532e47a14dba7603c9ab0598e68be2c0822200855844edd45f50bfba481451ca5ee5247dbca2b54fe522e74a658edc15c8eed917360e1a289b3ab78ecf4c@3.250.36.7:30311",
    "enode://9f005be9111a6152884fd575abb55bddb1e7f726510c96cddde57a9bba84ffa4952a89d7632c9c9dd50d3750f83966a73a0f7ed793f253a3691b84a687b29b6c@3.88.177.211:30311",
    "enode://5451251a9902e658154456ea98ebdd93313e54496ce0a6ca2242fe4db882940d78d758c85a36485af54b0841270f2bdbff64d66c45976f3ed1dd912f7649c831@3.236.189.129:30311",
    "enode://a232f92d1e76447b93306ece2f6a55ac70ca4633fae0938d71a100757eaf8526e6bbf720aa70cba1e6d186be17291ad1ee851a35596ec6caa2fdf135ce4b6b68@107.20.124.16:30311",
    "enode://62c516645635f0389b4c851bfc4545720fac0607de74942e4ea7e923f4fa2ac0c438c146e2f0721c8ce06dca4e7f30f5c0136569d9f4b6a827c62b980fd53272@52.215.57.20:30311",
    "enode://c014bbf48209cdf8ca6d3bf3ff5cf2fade45104283dcfc079df6c64e0f4b65e4afe28040fa1731a0732bd9cbb90786cf78f0174b5de7bd5b303088e80d8e6a83@54.74.101.143:30311",
    "enode://710ed272e03b92c803cd165e5fa071da015815d312f17d107a43ad3b12b0f05c830c58ced2df7547294f5365fe76cdcf1a58f923ee5612d247a6d5b80cfe16a8@34.245.31.55:30311",
    "enode://768af449287561c0f17bb5dc5d98a1c6a4b1798cb41159bd0a7bfebdc179e39ad8076d7292caa9344eecb94a5f7499e632c29cc4edbdf2e8ada3f7c8c7b2a64b@3.95.173.72:30311",
    "enode://8428650e034341479d0ca3142bcd412f400ba47454bb7caeb88cfeb9bb60c21e45153eddf3e334d5d94ae67609ec2ac44816b346a2b3216d94a7c095883141e3@54.195.188.155:30311",
    ];

pub const OUR_NODE: [&str; 1]= ["enode://e64736846307a7935165c8152659b6fc0a722fd420b3c4214ce283dc17ed25961e98fac951386a57a8a055fcd8222323a928d11ae665a87da190f561b9578567@127.0.0.1:30311"];
