/// There are four blockchains: mainnet, testnet, stn, and regtest.
#[derive(Copy, Clone)]
pub enum Blockchain {
    Mainnet = 0,
    Testnet = 1,
    Stn = 2,
    Regtest = 3,
}
