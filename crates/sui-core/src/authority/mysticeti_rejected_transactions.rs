// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use consensus_core::Round;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

use mysten_common::sync::notify_read::NotifyRead;
use sui_types::error::SuiError;

use crate::wait_for_effects_request::MysticetiTransactionPosition;

// TODO: Figure out the proper value for this.
const ROUND_EXPIRATION: Round = 100;

#[derive(Default)]
pub(crate) struct MysticetiRejectedTransactions {
    inner: RwLock<Inner>,
    status_notify_read: NotifyRead<MysticetiTransactionPosition, ()>,
}

#[derive(Default)]
struct Inner {
    /// All transactions that have been rejected by mysticeti,
    /// either due to fast-path reject or post-commit reject.
    rejected_transactions: HashSet<MysticetiTransactionPosition>,
    /// A map of consensus round to all transactions that were rejected in that round.
    /// This is used to expire old rejected transactions and reclaim memory.
    round_lookup_map: BTreeMap<Round, HashSet<MysticetiTransactionPosition>>,
    /// The last round that was committed.
    last_committed_round: Option<Round>,
}

impl MysticetiRejectedTransactions {
    pub fn new() -> Self {
        Self::default()
    }

    // TODO: Propagate the reason for rejection.
    pub fn reject_transaction(&self, transaction_position: MysticetiTransactionPosition) {
        let mut inner = self.inner.write();
        if let Some(last_committed_round) = inner.last_committed_round {
            if transaction_position.block_ref.round + ROUND_EXPIRATION < last_committed_round {
                return;
            }
        }
        inner
            .rejected_transactions
            .insert(transaction_position.clone());
        inner
            .round_lookup_map
            .entry(transaction_position.block_ref.round)
            .or_default()
            .insert(transaction_position.clone());
        self.status_notify_read.notify(&transaction_position, &());
    }

    /// Wait for a transaction to be rejected through mysticeti.
    /// Returns error when one of the following happens:
    /// 1. If the transaction has already been rejected
    /// 2. If the transaction gets rejected after waiting for some time.
    /// 3. If we have waited for the duration without being notified.
    /// 4. If the transaction is too old comparing to the last committed round.
    ///
    /// Note: This function always return an error. This is a design choice that would allow
    /// us to propagate the reason for rejection to the caller in the future.
    pub async fn wait_for_rejection(
        &self,
        transaction_position: MysticetiTransactionPosition,
        duration: Duration,
    ) -> SuiError {
        let registration = self.status_notify_read.register_one(&transaction_position);
        if self
            .inner
            .read()
            .rejected_transactions
            .contains(&transaction_position)
        {
            return SuiError::TransactionRejectedByConsensus {
                reason: "Rejectd".to_string(),
            };
        }
        let expiration_check = async {
            loop {
                {
                    let inner = self.inner.read();
                    if let Some(last_committed_round) = inner.last_committed_round {
                        if transaction_position.block_ref.round + ROUND_EXPIRATION
                            < last_committed_round
                        {
                            return;
                        }
                    }
                }
                sleep(Duration::from_millis(50)).await;
            }
        };
        tokio::select! {
            _ = registration => SuiError::TransactionRejectedByConsensus {
                reason: "Rejected".to_string(),
            },
            _ = expiration_check => SuiError::TransactionRejectedByConsensus {
                reason: "Expired".to_string(),
            },
            _ = sleep(duration) => SuiError::TransactionRejectedByConsensus {
                reason: "TimedOut".to_string(),
            },
        }
    }

    pub async fn update_last_committed_round(&self, round: Round) {
        debug!("Updating last committed round: {}", round);
        let mut inner = self.inner.write();
        while let Some(&next_round) = inner.round_lookup_map.keys().next() {
            if next_round + ROUND_EXPIRATION < round {
                let transactions = inner.round_lookup_map.remove(&next_round).unwrap();
                for tx in transactions {
                    inner.rejected_transactions.remove(&tx);
                }
            } else {
                break;
            }
        }
        inner.last_committed_round = Some(round);
    }
}

#[cfg(test)]
mod tests {
    use consensus_config::AuthorityIndex;
    use consensus_core::BlockRef;

    use super::*;
    use std::time::Duration;

    fn create_test_position(round: Round, transaction_index: u32) -> MysticetiTransactionPosition {
        MysticetiTransactionPosition {
            block_ref: BlockRef::new(round, AuthorityIndex::new_for_test(0), Default::default()),
            transaction_index,
        }
    }

    #[tokio::test]
    async fn test_reject_transaction() {
        let rejected_txs = MysticetiRejectedTransactions::new();
        let pos = create_test_position(1, 0);

        rejected_txs.reject_transaction(pos.clone());

        let inner = rejected_txs.inner.read();
        assert!(inner.rejected_transactions.contains(&pos));
        assert!(inner.round_lookup_map.get(&1).unwrap().contains(&pos));
    }

    #[tokio::test]
    async fn test_wait_for_rejection() {
        let rejected_txs = MysticetiRejectedTransactions::new();
        let pos = create_test_position(1, 0);

        // Test immediate rejection
        rejected_txs.reject_transaction(pos.clone());
        let result = rejected_txs
            .wait_for_rejection(pos.clone(), Duration::from_secs(1))
            .await;
        assert!(matches!(
            result,
            SuiError::TransactionRejectedByConsensus { reason } if reason == "Rejected"
        ));

        // Test timeout
        let result = rejected_txs
            .wait_for_rejection(pos, Duration::from_millis(100))
            .await;
        assert!(matches!(
            result,
            SuiError::TransactionRejectedByConsensus { reason } if reason == "TimedOut"
        ));
    }

    #[tokio::test]
    async fn test_round_expiration() {
        let rejected_txs = MysticetiRejectedTransactions::new();
        let pos = create_test_position(1, 0);

        rejected_txs.reject_transaction(pos.clone());

        // Update to a round that would cause expiration
        rejected_txs
            .update_last_committed_round(ROUND_EXPIRATION + 2)
            .await;

        let result = rejected_txs
            .wait_for_rejection(pos.clone(), Duration::from_secs(5))
            .await;
        assert!(matches!(
            result,
            SuiError::TransactionRejectedByConsensus { reason } if reason == "Expired"
        ));

        // Try to reject a transaction from an expired round
        rejected_txs.reject_transaction(pos);

        let inner = rejected_txs.inner.read();
        assert!(inner.round_lookup_map.is_empty());
        assert!(inner.rejected_transactions.is_empty());
    }

    #[tokio::test]
    async fn test_update_last_committed_round() {
        let rejected_txs = MysticetiRejectedTransactions::new();

        // Add transactions for multiple rounds
        for round in 1..=5 {
            let pos = create_test_position(round, 0);
            rejected_txs.reject_transaction(pos);
        }

        // Update to round that would expire rounds 1 and 2
        let expiration_round = ROUND_EXPIRATION + 3;
        rejected_txs
            .update_last_committed_round(expiration_round)
            .await;

        let inner = rejected_txs.inner.read();
        assert_eq!(inner.last_committed_round, Some(expiration_round));

        // Rounds 1 and 2 should be removed
        assert!(!inner.round_lookup_map.contains_key(&1));
        assert!(!inner.round_lookup_map.contains_key(&2));

        // Rounds 3, 4, and 5 should still exist
        assert!(inner.round_lookup_map.contains_key(&3));
        assert!(inner.round_lookup_map.contains_key(&4));
        assert!(inner.round_lookup_map.contains_key(&5));
    }
}

// TODO: Add tests
