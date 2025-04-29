// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use mysten_metrics::spawn_monitored_task;
use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
};
use sui_config::node::AuthorityOverloadConfig;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

use sui_types::{
    base_types::FullObjectID,
    digests::TransactionEffectsDigest,
    error::SuiResult,
    executable_transaction::VerifiedExecutableTransaction,
    storage::InputKey,
    transaction::{SenderSignedData, TransactionDataAPI, VerifiedCertificate},
};

use crate::{
    authority::authority_per_epoch_store::AuthorityPerEpochStore,
    execution_cache::{ObjectCacheRead, TransactionCacheRead},
    transaction_manager::{PendingCertificate, PendingCertificateStats},
};

#[derive(Clone)]
pub struct TransactionManagerV2 {
    object_cache_read: Arc<dyn ObjectCacheRead>,
    transaction_cache_read: Arc<dyn TransactionCacheRead>,
    tx_ready_certificates: UnboundedSender<PendingCertificate>,
}

impl TransactionManagerV2 {
    pub fn new(
        object_cache_read: Arc<dyn ObjectCacheRead>,
        transaction_cache_read: Arc<dyn TransactionCacheRead>,
        tx_ready_certificates: UnboundedSender<PendingCertificate>,
    ) -> Self {
        Self {
            object_cache_read,
            transaction_cache_read,
            tx_ready_certificates,
        }
    }

    pub(crate) fn enqueue_with_expected_effects_digest(
        &self,
        certs: Vec<(VerifiedExecutableTransaction, TransactionEffectsDigest)>,
        epoch_store: &Arc<AuthorityPerEpochStore>,
    ) {
        let certs = certs
            .into_iter()
            .map(|(cert, fx)| (cert, Some(fx)))
            .collect();
        self.enqueue_impl(certs, epoch_store)
    }

    pub(crate) fn enqueue_certificates(
        &self,
        certs: Vec<VerifiedCertificate>,
        epoch_store: &Arc<AuthorityPerEpochStore>,
    ) {
        let executable_txns = certs
            .into_iter()
            .map(VerifiedExecutableTransaction::new_from_certificate)
            .collect();
        self.enqueue(executable_txns, epoch_store)
    }

    pub(crate) fn enqueue(
        &self,
        certs: Vec<VerifiedExecutableTransaction>,
        epoch_store: &Arc<AuthorityPerEpochStore>,
    ) {
        let certs = certs.into_iter().map(|cert| (cert, None)).collect();
        self.enqueue_impl(certs, epoch_store)
    }

    fn enqueue_impl(
        &self,
        certs: Vec<(
            VerifiedExecutableTransaction,
            Option<TransactionEffectsDigest>,
        )>,
        epoch_store: &Arc<AuthorityPerEpochStore>,
    ) {
        let certs = certs.into_iter().filter_map(|cert| {
            if cert.0.epoch() == epoch_store.epoch() {
                Some(cert)
            } else {
                None
            }
        });

        for cert in certs {
            let tx_manager = self.clone();
            let epoch_store = epoch_store.clone();
            spawn_monitored_task!(
                epoch_store.within_alive_epoch(tx_manager.schedule_transaction(
                    cert.0,
                    cert.1,
                    &epoch_store
                ))
            );
        }
    }

    async fn schedule_transaction(
        self,
        cert: VerifiedExecutableTransaction,
        expected_effects_digest: Option<TransactionEffectsDigest>,
        epoch_store: &AuthorityPerEpochStore,
    ) {
        let enqueue_time = Instant::now();
        let tx_data = cert.transaction_data();
        let input_object_kinds = tx_data
            .input_objects()
            .expect("input_objects() cannot fail");

        let input_object_keys: Vec<_> =
            match epoch_store.get_input_object_keys(&cert.key(), &input_object_kinds) {
                Ok(keys) => keys,
                Err(_) => {
                    // This is possible if the transaction is already executed.
                    // TODO: Add assertions.
                    return;
                }
            }
            .into_iter()
            .collect();
        let receiving_object_keys: HashSet<_> = tx_data
            .receiving_objects()
            .into_iter()
            .map(|entry| {
                InputKey::VersionedObject {
                    // TODO: Add support for receiving ConsensusV2 objects. For now this assumes fastpath.
                    id: FullObjectID::new(entry.0, None),
                    version: entry.1,
                }
            })
            .collect();

        let input_and_receiving_keys = [
            input_object_keys,
            receiving_object_keys.iter().cloned().collect(),
        ]
        .concat();

        let epoch = epoch_store.epoch();
        let digests = [*cert.digest()];

        tokio::select! {
            _ = self.object_cache_read
                .notify_read_input_objects(&input_and_receiving_keys, &receiving_object_keys, &epoch)
                => {
                let pending_cert = PendingCertificate {
                    certificate: cert,
                    expected_effects_digest,
                    waiting_input_objects: BTreeSet::new(),
                    stats: PendingCertificateStats {
                        enqueue_time,
                        ready_time: Some(Instant::now()),
                    },
                };
                self.tx_ready_certificates.send(pending_cert).unwrap();
            }
            _ = self.transaction_cache_read.notify_read_executed_effects(&digests) => {
            }
        };
    }

    pub(crate) fn check_execution_overload(
        &self,
        _overload_config: &AuthorityOverloadConfig,
        _tx_data: &SenderSignedData,
    ) -> SuiResult {
        Ok(())
    }
}
