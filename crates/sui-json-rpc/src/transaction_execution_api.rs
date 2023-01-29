// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::api::TransactionExecutionApiServer;
use crate::SuiRpcModule;
use anyhow::anyhow;
use async_trait::async_trait;
use fastcrypto::encoding::Base64;
use jsonrpsee::core::RpcResult;
use jsonrpsee::RpcModule;
use move_bytecode_utils::module_cache::SyncModuleCache;
use mysten_metrics::spawn_monitored_task;
use signature::Signature;
use std::sync::Arc;
use sui_core::authority::{AuthorityState, AuthorityStore, ResolverWrapper};
use sui_core::authority_client::NetworkAuthorityClient;
use sui_core::transaction_orchestrator::TransactiondOrchestrator;
use sui_json_rpc_types::{
    SuiCertifiedTransaction, SuiCertifiedTransactionEffects, SuiExecuteTransactionResponse,
    SuiTransactionEvents,
};
use sui_open_rpc::Module;
use sui_types::intent::Intent;
use sui_types::messages::{
    ExecuteTransactionRequest, ExecuteTransactionRequestType, ExecuteTransactionResponse,
};
use sui_types::{crypto, messages::Transaction};

pub struct FullNodeTransactionExecutionApi {
    pub state: Arc<AuthorityState>,
    pub transaction_orchestrator: Arc<TransactiondOrchestrator<NetworkAuthorityClient>>,
    pub module_cache: Arc<SyncModuleCache<ResolverWrapper<AuthorityStore>>>,
}

impl FullNodeTransactionExecutionApi {
    pub fn new(
        state: Arc<AuthorityState>,
        transaction_orchestrator: Arc<TransactiondOrchestrator<NetworkAuthorityClient>>,
        module_cache: Arc<SyncModuleCache<ResolverWrapper<AuthorityStore>>>,
    ) -> Self {
        Self {
            state,
            transaction_orchestrator,
            module_cache,
        }
    }
}

#[async_trait]
impl TransactionExecutionApiServer for FullNodeTransactionExecutionApi {
    async fn execute_transaction(
        &self,
        tx_bytes: Base64,
        signature: Base64,
        request_type: ExecuteTransactionRequestType,
    ) -> RpcResult<SuiExecuteTransactionResponse> {
        let tx_data =
            bcs::from_bytes(&tx_bytes.to_vec().map_err(|e| anyhow!(e))?).map_err(|e| anyhow!(e))?;
        let signature = crypto::Signature::from_bytes(&signature.to_vec().map_err(|e| anyhow!(e))?)
            .map_err(|e| anyhow!(e))?;

        let txn = Transaction::from_data(tx_data, Intent::default(), signature);

        let transaction_orchestrator = self.transaction_orchestrator.clone();
        let response = spawn_monitored_task!(transaction_orchestrator.execute_transaction(
            ExecuteTransactionRequest {
                transaction: txn,
                request_type,
            }
        ))
        .await
        .map_err(|e| anyhow!(e))? // for JoinError
        .map_err(|e| anyhow!(e))?; // For Sui transaction execution error (SuiResult<ExecuteTransactionResponse>)

        Ok(match response {
            ExecuteTransactionResponse::EffectsCert(cert) => {
                let (certificate, effects, is_executed_locally) = *cert;
                let certificate: SuiCertifiedTransaction = certificate.try_into()?;
                let effects: SuiCertifiedTransactionEffects = effects.try_into()?;
                let events = SuiTransactionEvents::try_from(
                    self.state
                        .get_transaction_events(effects.effects.events_digest)
                        .await?,
                    self.state.module_cache.as_ref(),
                )?;
                SuiExecuteTransactionResponse::EffectsCert {
                    certificate,
                    effects,
                    events,
                    confirmed_local_execution: is_executed_locally,
                }
            }
        })
    }

    async fn execute_transaction_serialized_sig(
        &self,
        tx_bytes: Base64,
        signature: Base64,
        request_type: ExecuteTransactionRequestType,
    ) -> RpcResult<SuiExecuteTransactionResponse> {
        self.execute_transaction(tx_bytes, signature, request_type)
            .await
    }
}

impl SuiRpcModule for FullNodeTransactionExecutionApi {
    fn rpc(self) -> RpcModule<Self> {
        self.into_rpc()
    }

    fn rpc_doc_module() -> Module {
        crate::api::TransactionExecutionApiOpenRpc::module_doc()
    }
}
