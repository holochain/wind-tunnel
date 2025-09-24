use hdk::prelude::*;
use holochain_serialized_bytes::SerializedBytes;
use serde::{Deserialize, Serialize};
use validated_must_get_agent_activity_integrity::{
    EntryTypes, LinkTypes, SampleEntry, ValidatedSampleEntry,
};

fn chain_head_anchor() -> ExternResult<AnyLinkableHash> {
    Ok(Path::from("LATEST_ENTRY").path_entry_hash()?.into())
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
struct ChainLenTag(pub usize);

#[hdk_extern]
fn create_sample_entries_batch(count: u64) -> ExternResult<()> {
    // Create batch entries
    let mut action_hashes: Vec<ActionHash> = vec![];
    for _ in 0..count {
        action_hashes.push(create_entry(EntryTypes::SampleEntry(SampleEntry {
            value: "This is a sample entry".to_string(),
        }))?);
    }

    // Query my chain and count length
    let chain_len = query(ChainQueryFilter::new().include_entries(false))?.len();
    let serialized_bytes: SerializedBytes = ChainLenTag(chain_len).try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(
            "Failed to convert chain len to serialized bytes".into()
        ))
    })?;

    // Link to last created action hash
    if let Some(last_ah) = action_hashes.last() {
        let _ = create_link(
            chain_head_anchor()?,
            last_ah.clone(),
            LinkTypes::LatestAction,
            serialized_bytes.bytes().clone(),
        )?;
    }

    Ok(())
}

#[hdk_extern]
fn create_validated_sample_entry(agent: AgentPubKey) -> ExternResult<usize> {
    // Get last created action hash
    let mut links: Vec<Link> = get_links(
        GetLinksInputBuilder::try_new(chain_head_anchor()?, LinkTypes::LatestAction)?.build(),
    )?;
    links.sort_by_key(|l| l.timestamp);
    let chain_head_link: Link = links
        .last()
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(String::from(
                "No chain head link found"
            )))
        })?
        .clone();
    let chain_head = chain_head_link.target.try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(String::from(
            "Invalid link target type"
        )))
    })?;
    let serialized_bytes = SerializedBytes::from(UnsafeBytes::from(chain_head_link.tag.0));
    let chain_len_tag: ChainLenTag = serialized_bytes.try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(
            "Failed to deserialize tag to chain len".to_string()
        ))
    })?;

    // Create entry that validates with must_get_agent_activity
    let _ = create_entry(EntryTypes::ValidatedSampleEntry(ValidatedSampleEntry {
        agent,
        chain_head,
        chain_len: chain_len_tag.0,
    }))?;

    Ok(chain_len_tag.0)
}
