use hdk::prelude::*;
use timed_and_validated_integrity::{fixed_base, EntryTypes, LinkTypes, TimedSampleEntry};

#[hdk_extern]
fn create_timed_entry(timed: TimedSampleEntry) -> ExternResult<ActionHash> {
    let action_hash = create_entry(EntryTypes::TimedSampleEntry(timed))?;

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
    // No way to control whether this goes to the network at this HDK version
    let links = get_links(
        GetLinksInputBuilder::try_new(fixed_base(), LinkTypes::FixedToTimedEntry)
            .unwrap()
            .build(),
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
    // No way to control whether this goes to the network at this HDK version
    let links = get_links(
        GetLinksInputBuilder::try_new(fixed_base(), LinkTypes::FixedToTimedEntry)
            .unwrap()
            .build(),
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
