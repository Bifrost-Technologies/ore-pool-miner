use solana_client::nonblocking::rpc_client::RpcClient;

use crate::{utils::proof_pubkey, MINING_POOL};

    #[no_mangle]
    pub async fn open(rpc: &RpcClient) {
        
        let rpc_client: &RpcClient = rpc;

        let proof_address = proof_pubkey(MINING_POOL);
        if rpc_client.get_account(&proof_address).await.is_ok() {
            return;
        }

    }

