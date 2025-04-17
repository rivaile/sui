// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

module mp::mp {
    use std::vector;
    use sui::clock;

    public struct Obj has key {
        id: object::UID,
    }

    /// Create a single-owner multiparty object and tranfer to the sender.
    public fun create_multiparty(ctx: &mut TxContext) {
        transfer::multiparty_transfer(
            Obj {
                id: object::new(ctx),
            },
            ctx.sender(),
        );
    }

    /// Create a single-owner fastpath object and transfer to the sender.
    public fun create_fastpath(ctx: &mut TxContext) {
        transfer::transfer(
            Obj {
                id: object::new(ctx),
            },
            ctx.sender(),
        );
    }

    /// Transfer an object to a multiparty owner.
    public fun transfer_multiparty(obj: Obj, recipient: address) {
        transfer::multiparty_transfer(obj, recipient);
    }

    /// Transfer an object to a fastpath owner.
    public fun transfer_fastpath(obj: Obj, recipient: address) {
        transfer::transfer(obj, recipient);
    }
}
