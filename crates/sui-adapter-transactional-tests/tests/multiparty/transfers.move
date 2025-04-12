// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//# init --accounts A B --addresses ex=0x0

//# publish
module ex::m;

public struct Pub has key, store {
    id: UID,
}

public struct Priv has key {
    id: UID,
}

public fun mint(ctx: &mut TxContext) {
    let p = Pub { id: object::new(ctx) };
    let q = Priv { id: object::new(ctx) };
    transfer::public_transfer(p, ctx.sender());
    transfer::transfer(q, ctx.sender());
}

public fun create_multiparty(ctx: &mut TxContext) {
    let p = Pub { id: object::new(ctx) };
    transfer::public_multiparty_transfer(p, sui::multiparty::single_owner(@0))
}

public fun pub_multiparty(obj: Pub, p: sui::multiparty::Multiparty) {
    transfer::public_multiparty_transfer(obj, p)
}

public fun priv_multiparty(obj: Priv, p: sui::multiparty::Multiparty) {
    transfer::multiparty_transfer(obj, p)
}

public fun priv_fastpath(obj: Priv, addr: address) {
    transfer::transfer(obj, addr)
}

//# run ex::m::mint

// Creates a multiparty object using `public_multiparty_transfer` on a struct with store
//# run ex::m::create_multiparty

// This is the `Priv` object.
//# view-object 2,0

// This is the `Pub` object.
//# view-object 2,1

// Transfers from fastpath to multiparty via `multiparty_transfer` on a struct without store
//# programmable --inputs object(2,0) @A
//> 0: sui::multiparty::single_owner(Input(1));
//> ex::m::priv_multiparty(Input(0), Result(0))

//# view-object 2,0

// Transfers from fastpath to multiparty via `public_multiparty_transfer` on a struct with store
//# programmable --inputs object(2,1) @A
//> 0: sui::multiparty::single_owner(Input(1));
//> ex::m::pub_multiparty(Input(0), Result(0))

//# view-object 2,1

// Transfers a multiparty object with wrong sender; should fail
//# programmable --inputs object(2,1) @A --sender B
//> 0: sui::multiparty::single_owner(Input(1));
//> sui::transfer::public_multiparty_transfer<ex::m::Pub>(Input(0), Result(0))

// Transfers an existing multiparty object back to multiparty again
//# programmable --inputs object(2,1) @A --sender A
//> 0: sui::multiparty::single_owner(Input(1));
//> sui::transfer::public_multiparty_transfer<ex::m::Pub>(Input(0), Result(0))

//# view-object 2,1

// Transfers an existing multiparty object to a different owner; start_version should stay the same
//# programmable --inputs object(2,1) @B --sender A
//> 0: sui::multiparty::single_owner(Input(1));
//> sui::transfer::public_multiparty_transfer<ex::m::Pub>(Input(0), Result(0))

//# view-object 2,1

// Transfers multiparty object using transfer-object with wrong sender; should fail
//# transfer-object 2,1 --sender A --recipient A

// Transfers multiparty object back to fastpath using transfer-object
//# transfer-object 2,1 --sender B --recipient B

//# view-object 2,1

// Transfers multiparty object via `transfer` with wrong sender; should fail 
//# programmable --inputs object(2,0) @B --sender B
//> ex::m::priv_fastpath(Input(0), Input(1));

// Transfers multiparty object back to fastpath and a different owner via `transfer`
//# programmable --inputs object(2,0) @B --sender A
//> ex::m::priv_fastpath(Input(0), Input(1));

//# view-object 2,0
