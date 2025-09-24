use hdk::prelude::*;
use std::cell::LazyCell;
use validated_must_get_agent_activity_integrity::{
    EntryTypes, LinkTypes, SampleEntry, ValidatedSampleEntry,
};

const CHAIN_HEAD_ANCHOR: LazyCell<AnyLinkableHash> =
    LazyCell::new(|| Path::from("LATEST_ENTRY").path_entry_hash().unwrap().into());

#[hdk_extern]
fn create_sample_entries_batch(count: u64) -> ExternResult<usize> {
    // Create batch entries
    let mut action_hashes: Vec<ActionHash> = vec![];
    for _ in 0..count {
        action_hashes.push(create_entry(EntryTypes::SampleEntry(SampleEntry {
            value: "This is a sample entry".to_string(),
        }))?);
    }

    // Query my chain and
    let chain_len = query(ChainQueryFilter::new().include_entries(false))?.len();

    // Link to last created action hash
    if let Some(last_ah) = action_hashes.last() {
        let _ = create_link(
            CHAIN_HEAD_ANCHOR.clone(),
            last_ah.clone(),
            LinkTypes::LatestAction,
            (),
        )?;
    }

    Ok(chain_len)
}

#[hdk_extern]
fn create_validated_sample_entry(agent: AgentPubKey) -> ExternResult<ActionHash> {
    // Get last created action hash
    let mut links: Vec<Link> = get_links(
        GetLinksInputBuilder::try_new(CHAIN_HEAD_ANCHOR.clone(), LinkTypes::LatestAction)?.build(),
    )?;
    links.sort_by_key(|l| l.timestamp);
    let chain_head: ActionHash = links
        .last()
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(String::from(
                "No chain head link found"
            )))
        })?
        .clone()
        .target
        .try_into()
        .map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(String::from(
                "Invalid link target type"
            )))
        })?;

    // Create entry that validates with must_get_agent_activity
    create_entry(EntryTypes::ValidatedSampleEntry(ValidatedSampleEntry {
        agent,
        chain_head,
    }))
}
