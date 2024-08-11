mod busses;
mod open;
mod utils;
use colored::*;
use drillx::{
    equix::{self},
    Hash, Solution,
};
use ore_api::{
    consts::{BUS_ADDRESSES, BUS_COUNT, EPOCH_DURATION},
    state::{Config, Proof},
};
use rand::Rng;
use reqwest::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::spinner;
use std::time::Duration;
use std::{sync::Arc, time::Instant};

use crate::open::open;
use crate::utils::{amount_u64_to_string, get_clock, get_config, get_updated_proof_with_authority};

//Default is Alvarium Mining Pool. You can replace with a different mining pool address
pub const MINING_POOL: Pubkey = solana_program::pubkey!("Cdh9QF6NmxCxWDEmuusFVkhQSZuVMRXj9nnZQyGraCna");

//MUST BE CHANGED TO RECEIVE PAYOUT. Use your wallet address here
pub const MINER_PAYOUT_ADDRESS: Pubkey = solana_program::pubkey!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");

//Replace with your favorite RPC provider
pub const MINING_POOL_RPC: &str = "REPLACE WITH RPC PROVIDER";

//Default is Alvarium Mining Pool. Change this to your pool's API endpoint
pub const MINING_POOL_URL: &str = "https://alvarium.bifrost.technology/submitwork";

//Update amount of threads
pub const THREADS: u64 = 50;

//Keep at 5 for Alvarium Mining Pool. Change this value if the pool requires you to hand in the work sooner
pub const POOL_BUFFER: u64 = 5;


#[tokio::main]
async fn main() {
    let mut rng = rand::thread_rng();

    let random_depth = rng.gen_range(50..=500);

    let settings = Arc::new(Minersettings {
        _threads: THREADS,
        _buffer: POOL_BUFFER,
        _depth: random_depth,
        _miner: MINER_PAYOUT_ADDRESS,
    });

    mine(settings).await;
}

#[no_mangle]
pub extern "C" fn minepool(_threads: u64, _buffer: u64) {
    //run through runtime (optional for C lib)
    //mine(settings);
}

#[no_mangle]
pub async fn mine(data: Arc<Minersettings>) {
    open().await;
    let mut _previous_challenge: String = String::new();
    let mut _current_challenge: String = String::new();
    loop {
        let rpc_client: RpcClient = RpcClient::new(MINING_POOL_RPC.to_string());
        let last_hash_at = 0;
        let proof = get_updated_proof_with_authority(&rpc_client, MINING_POOL, last_hash_at).await;

        println!(
            "\n Mining Pool Stake balance: {} ORE",
            amount_u64_to_string(proof.balance)
        );
        _current_challenge = bs58::encode(proof.challenge.as_slice()).into_string();
        println!("Current Challenge: {}", _current_challenge);
        if _current_challenge != _previous_challenge {

            // Calc cutoff time
            let cutoff_time = get_cutoff(proof, data._buffer).await;

            // Run drillx
            let config = get_config(&rpc_client).await;
            let (solution, _best_difficulty): (Solution, u32) = find_hash_par(
                proof,
                cutoff_time,
                data._threads,
                data._depth,
                config.min_difficulty as u32,
            )
            .await;

            //Serialize work as a B58 hash
            let workhash: Vec<u8> = [
                solution.d.as_slice(),
                solution.n.as_slice(),
                data._miner.to_bytes().as_slice(),
                proof.challenge.as_slice(),
                _best_difficulty.to_le_bytes().as_slice(),
            ]
            .concat();

            let webclient = Client::new();
            //Send the work to the pool to score your work and get payment
            let _response = webclient
                .post(MINING_POOL_URL)
                .json(&bs58::encode(workhash).into_string())
                .send()
                .await;
            _previous_challenge = _current_challenge;
        } else {
            println!("Waiting for new work...");
            std::thread::sleep(Duration::from_millis(5000));
        }
    }
}

#[no_mangle]
pub async fn find_hash_par(
    proof: Proof,
    cutoff_time: u64,
    threads: u64,
    depth: u64,
    min_difficulty: u32,
) -> (Solution, u32) {
    // Dispatch job to each thread
    let progress_bar = Arc::new(spinner::new_progress_bar());
    progress_bar.set_message("Mining...");
    let handles: Vec<_> = (0..threads)
        .map(|i| {
            std::thread::spawn({
                let proof = proof.clone();
                let progress_bar = progress_bar.clone();
                let mut memory = equix::SolverMemory::new();
                move || {
                    let timer = Instant::now();
                    let mut nonce = u64::MAX.saturating_div(depth).saturating_mul(i);
                    let mut best_nonce = nonce;
                    let mut best_difficulty = 0;
                    let mut best_hash = Hash::default();
                    loop {
                        // Create hash
                        if let Ok(hx) = drillx::hash_with_memory(
                            &mut memory,
                            &proof.challenge,
                            &nonce.to_le_bytes(),
                        ) {
                            let difficulty = hx.difficulty();
                            if difficulty.gt(&best_difficulty) {
                                best_nonce = nonce;
                                best_difficulty = difficulty;
                                best_hash = hx;
                            }
                        }

                        // Exit if time has elapsed
                        if nonce % 100 == 0 {
                            if timer.elapsed().as_secs().ge(&cutoff_time) {
                                if best_difficulty.gt(&min_difficulty) {
                                    // Mine until min difficulty has been met
                                    break;
                                }
                            } else if i == 0 {
                                progress_bar.set_message(format!(
                                    "Mining... ({} sec remaining)",
                                    cutoff_time.saturating_sub(timer.elapsed().as_secs()),
                                ));
                            }
                        }

                        // Increment nonce
                        nonce += 1;
                    }

                    // Return the best nonce
                    (best_nonce, best_difficulty, best_hash)
                }
            })
        })
        .collect();

    // Join handles and return best nonce
    let mut best_nonce = 0;
    let mut best_difficulty = 0;
    let mut best_hash = Hash::default();
    for h in handles {
        if let Ok((nonce, difficulty, hash)) = h.join() {
            if difficulty > best_difficulty {
                best_difficulty = difficulty;
                best_nonce = nonce;
                best_hash = hash;
            }
        }
    }

    // Update log
    progress_bar.finish_with_message(format!(
        "Best hash: {} (difficulty: {})",
        bs58::encode(best_hash.h).into_string(),
        best_difficulty
    ));

    (
        Solution::new(best_hash.d, best_nonce.to_le_bytes()),
        best_difficulty,
    )
}
pub struct Minersettings {
    _threads: u64,
    _buffer: u64,
    _depth: u64,
    _miner: Pubkey,
}
unsafe impl Send for Minersettings {}

#[no_mangle]
pub fn check_num_cores(threads: u64) {
    // Check num threads
    let num_cores = num_cpus::get() as u64;
    if threads.gt(&num_cores) {
        println!(
            "{} Number of threads ({}) exceeds available cores ({})",
            "WARNING".bold().yellow(),
            threads,
            num_cores
        );
    }
}

#[no_mangle]
pub async fn should_reset(config: Config) -> bool {
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc_client: RpcClient = RpcClient::new(rpc_url.to_string());
    let clock = get_clock(&rpc_client).await;
    config
        .last_reset_at
        .saturating_add(EPOCH_DURATION)
        .saturating_sub(5) // Buffer
        .le(&clock.unix_timestamp)
}

#[no_mangle]
pub async fn get_cutoff(proof: Proof, buffer_time: u64) -> u64 {
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc_client: RpcClient = RpcClient::new(rpc_url.to_string());
    let clock = get_clock(&rpc_client).await;
    proof
        .last_hash_at
        .saturating_add(60)
        .saturating_sub(buffer_time as i64)
        .saturating_sub(clock.unix_timestamp)
        .max(0) as u64
}

// TODO Pick a better strategy (avoid draining bus)
#[no_mangle]
pub fn find_bus() -> Pubkey {
    let i = rand::thread_rng().gen_range(0..BUS_COUNT);
    BUS_ADDRESSES[i]
}
