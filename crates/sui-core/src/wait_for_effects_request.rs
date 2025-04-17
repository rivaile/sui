// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use consensus_core::BlockRef;
use serde::{Deserialize, Serialize};
use sui_types::{
    digests::TransactionDigest,
    effects::{TransactionEffects, TransactionEvents},
    error::SuiError,
    messages_grpc::{RawWaitForEffectsRequest, RawWaitForEffectsResponse},
    object::Object,
};
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct MysticetiTransactionPosition {
    pub block_ref: BlockRef,
    pub transaction_index: u32,
}

pub(crate) struct WaitForEffectsRequest {
    pub transaction_digest: TransactionDigest,
    pub transaction_position: MysticetiTransactionPosition,
    pub include_events: bool,
    pub include_input_objects: bool,
    pub include_output_objects: bool,
}

pub(crate) struct WaitForEffectsResponse {
    pub effects: TransactionEffects,
    pub events: Option<TransactionEvents>,
    pub input_objects: Vec<Object>,
    pub output_objects: Vec<Object>,
}

impl TryFrom<RawWaitForEffectsRequest> for WaitForEffectsRequest {
    type Error = SuiError;

    fn try_from(value: RawWaitForEffectsRequest) -> Result<Self, Self::Error> {
        let transaction_digest = bcs::from_bytes(&value.transaction_digest)
            .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?;
        let transaction_position = bcs::from_bytes(&value.transaction_position)
            .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?;
        Ok(Self {
            transaction_digest,
            transaction_position,
            include_events: value.include_events,
            include_input_objects: value.include_input_objects,
            include_output_objects: value.include_output_objects,
        })
    }
}

impl TryFrom<RawWaitForEffectsResponse> for WaitForEffectsResponse {
    type Error = SuiError;

    fn try_from(value: RawWaitForEffectsResponse) -> Result<Self, Self::Error> {
        let effects = bcs::from_bytes(&value.effects)
            .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?;
        let events = if let Some(events) = value.events {
            Some(
                bcs::from_bytes(&events)
                    .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?,
            )
        } else {
            None
        };
        let mut input_objects = Vec::with_capacity(value.input_objects.len());
        for object in value.input_objects {
            input_objects.push(
                bcs::from_bytes(&object)
                    .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?,
            );
        }
        let mut output_objects = Vec::with_capacity(value.output_objects.len());
        for object in value.output_objects {
            output_objects.push(
                bcs::from_bytes(&object)
                    .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?,
            );
        }
        Ok(Self {
            effects,
            events,
            input_objects,
            output_objects,
        })
    }
}

impl TryFrom<WaitForEffectsRequest> for RawWaitForEffectsRequest {
    type Error = SuiError;

    fn try_from(value: WaitForEffectsRequest) -> Result<Self, Self::Error> {
        let transaction_digest = bcs::to_bytes(&value.transaction_digest)
            .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?
            .into();
        let transaction_position = bcs::to_bytes(&value.transaction_position)
            .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?
            .into();
        Ok(Self {
            transaction_digest,
            transaction_position,
            include_events: value.include_events,
            include_input_objects: value.include_input_objects,
            include_output_objects: value.include_output_objects,
        })
    }
}

impl TryFrom<WaitForEffectsResponse> for RawWaitForEffectsResponse {
    type Error = SuiError;

    fn try_from(value: WaitForEffectsResponse) -> Result<Self, Self::Error> {
        let effects = bcs::to_bytes(&value.effects)
            .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?
            .into();
        let events = if let Some(events) = value.events {
            Some(
                bcs::to_bytes(&events)
                    .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?
                    .into(),
            )
        } else {
            None
        };
        let mut input_objects = Vec::with_capacity(value.input_objects.len());
        for object in value.input_objects {
            input_objects.push(
                bcs::to_bytes(&object)
                    .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?
                    .into(),
            );
        }
        let mut output_objects = Vec::with_capacity(value.output_objects.len());
        for object in value.output_objects {
            output_objects.push(
                bcs::to_bytes(&object)
                    .map_err(|err| SuiError::GrpcMessageSerdeError(err.to_string()))?
                    .into(),
            );
        }
        Ok(Self {
            effects,
            events,
            input_objects,
            output_objects,
        })
    }
}
