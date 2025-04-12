// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//# init --accounts A B --addresses ex=0x0

//# publish
module ex::m;

public struct PubA has key, store {
    id: UID,
}

public struct PubB has key, store {
    id: UID,
}

public fun mint(ctx: &mut TxContext) {
        let fastpath_parent = PubA { id: object::new(ctx) };
        let fastpath_address = object::id_address(&fastpath_parent);
        let multiparty_parent = PubA { id: object::new(ctx) };
        let multiparty_address = object::id_address(&multiparty_parent);

        transfer::public_transfer(fastpath_parent, tx_context::sender(ctx));
        transfer::public_multiparty_transfer(multiparty_parent, sui::multiparty::single_owner(tx_context::sender(ctx)));

        let fastpath_child_fastpath_parent = PubB { id: object::new(ctx) };
        let fastpath_child_multiparty_parent = PubB { id: object::new(ctx) };

        transfer::public_transfer(fastpath_child_fastpath_parent, fastpath_address);
        transfer::public_multiparty_transfer(fastpath_child_multiparty_parent, sui::multiparty::single_owner(fastpath_address));

        let multiparty_child_fastpath_parent = PubB { id: object::new(ctx) };
        let multiparty_child_multiparty_parent = PubB { id: object::new(ctx) };

        transfer::public_transfer(multiparty_child_fastpath_parent, multiparty_address);
        transfer::public_multiparty_transfer(multiparty_child_multiparty_parent, sui::multiparty::single_owner(multiparty_address));
}

public entry fun receiver(parent: &mut PubA, x: sui::transfer::Receiving<PubB>) {
    let b = transfer::receive(&mut parent.id, x);
    transfer::public_transfer(b, @ex);
}

//# run ex::m::mint

// fastpath_parent
//# view-object 2,0

// multiparty_parent
//# view-object 2,1

// multiparty_child_multiparty_parent
//# view-object 2,2

// multiparty_child_fastpath_parent
//# view-object 2,3

// fastpath_child_fastpath_parent
//# view-object 2,4

// fastpath_child_multiparty_parent
//# view-object 2,5


// 1. Can receive a fastpath object from a fastpath parent.
//# run ex::m::receiver --args object(2,0) receiving(2,4)

//# view-object 2,4


// 2. Can receive a fastpath object from a multiparty parent.
//# run ex::m::receiver --args object(2,1) receiving(2,5)

//# view-object 2,5


// 3. Cannot receive a multiparty object from any parent type.

//# run ex::m::receiver --args object(2,0) receiving(2,3)

//# run ex::m::receiver --args object(2,1) receiving(2,2)
