use solana_client::nonblocking::rpc_client::RpcClient;

use crate::{utils::proof_pubkey, MINING_POOL, MINING_POOL_RPC};

    #[no_mangle]
    pub async fn open() {
        
        let rpc_client: RpcClient = RpcClient::new(MINING_POOL_RPC.to_string());

        let proof_address = proof_pubkey(MINING_POOL);
        if rpc_client.get_account(&proof_address).await.is_ok() {
            return;
        }

    }

