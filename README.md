# Ore Pool Miner
 Ore Pool Miner coded in Rust for the Ore v2 program on Solana

### Default Ore Mining Pool
Alvarium Mining Pool is operated by Bifrost and is the default pool option for the mining client.

### Custom Ore Mining Pool
There is no open source mining pool API template. A custom API server has to be built in order to use this client.

Here are some tools to get started: 

[Solnet.Ore](https://github.com/Bifrost-Technologies/Solnet.Ore) C# Ore SDK & Client

[Solnet](https://github.com/bmresearch/Solnet) C# Solana SDK & Client

## Install

Install Rustup for Windows to compile the miner with cargo
```
https://www.rust-lang.org/tools/install
```
## Dependencies

If you run into issues during installation, please install the following dependencies for your operating system and try again:

Linux Only
```
sudo apt-get install openssl pkg-config libssl-dev
```

## Install

To install the alvarium mining client run this command

```
cargo install alvarium-cli
```

## Run

Run this command to start the miner with your custom settings. Remove brackets and fill in the parameters
```
alvarium [RPC_LINK] [WALLET_ADDRESS] [THREADS] [BUFFER]
```
