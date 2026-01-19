use hdk::prelude::holo_hash::blake2b_256;
use hdk::prelude::holo_hash::hash_type::AnyLinkable;
use hdk::prelude::*;
use timed_integrity::{EntryTypes, LinkTypes, TimedEntry};

#[hdk_extern]
fn created_timed_entry(timed: TimedEntry) -> ExternResult<ActionHash> {
    let action_hash = create_entry(EntryTypes::TimedEntry(timed))?;

    create_link(
        fixed_base(),
        action_hash.clone(),
        LinkTypes::FixedToTimedEntry,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
fn get_timed_entries_local() -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(fixed_base(), LinkTypes::FixedToTimedEntry).unwrap(),
        GetStrategy::Local,
    )?;

    let mut records = Vec::new();
    for link in links {
        let action_hash: ActionHash = link
            .target
            .try_into()
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Not an action hash".to_string())))?;
        // Try to stay local
        let record = get(action_hash, GetOptions::local())?;
        if let Some(record) = record {
            records.push(record);
        }
    }

    Ok(records)
}

#[hdk_extern]
fn get_timed_entries_network() -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(fixed_base(), LinkTypes::FixedToTimedEntry).unwrap(),
        GetStrategy::Network,
    )?;

    let mut records = Vec::new();
    for link in links {
        let action_hash: ActionHash = link
            .target
            .try_into()
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Not an action hash".to_string())))?;
        let record = get(action_hash, GetOptions::network())?;
        if let Some(record) = record {
            records.push(record);
        }
    }

    Ok(records)
}

fn fixed_base() -> AnyLinkableHash {
    let mut result = blake2b_256("fixed".as_bytes());
    for _ in 0..4 {
        result.insert(0, 0);
    }
    AnyLinkableHash::from_raw_36_and_type(result, AnyLinkable::External)
}
