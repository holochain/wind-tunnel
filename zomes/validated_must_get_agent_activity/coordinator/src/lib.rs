use hdk::prelude::*;
use holochain_serialized_bytes::SerializedBytes;
use rand::prelude::SliceRandom;
use rand::rng;
use serde::{Deserialize, Serialize};
use validated_must_get_agent_activity_integrity::{
    EntryTypes, LinkTypes, SampleEntry, ValidatedSampleEntry,
};

fn chain_batch_anchor(agent: AgentPubKey, batch_num: u32) -> ExternResult<AnyLinkableHash> {
    Ok(Path::from(
        format!(
            "CHAIN_BATCH_ANCHOR.{}.{batch_num}",
            AgentPubKeyB64::from(agent)
        )
        .as_str(),
    )
    .path_entry_hash()?
    .into())
}

fn write_agents_anchor() -> ExternResult<EntryHash> {
    Path::from("WRITE_AGENTS").path_entry_hash()
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
struct ChainLenTag(pub usize);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateEntriesBatchInput {
    pub num_entries: u32,
    pub batch_num: u32,
}

#[hdk_extern]
fn create_sample_entries_batch(input: CreateEntriesBatchInput) -> ExternResult<()> {
    // Create batch entries
    let mut action_hashes: Vec<ActionHash> = vec![];
    for _ in 0..input.num_entries {
        action_hashes.push(create_entry(EntryTypes::SampleEntry(SampleEntry {
            value: "This is a sample entry".to_string(),
        }))?);
    }

    // Query my chain and count length
    let chain_len = query(ChainQueryFilter::new().include_entries(false))?.len();
    let serialized_chain_len: SerializedBytes =
        ChainLenTag(chain_len).try_into().map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Failed to convert chain len to serialized bytes".into()
            ))
        })?;

    // Link to last created action hash
    if let Some(last_ah) = action_hashes.last() {
        let _ = create_link(
            chain_batch_anchor(agent_info()?.agent_initial_pubkey, input.batch_num)?,
            last_ah.clone(),
            LinkTypes::LatestAction,
            serialized_chain_len.bytes().clone(),
        )?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetChainTopForBatchInput {
    pub agent: AgentPubKey,
    pub batch_num: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatchChainTop {
    pub chain_top: ActionHash,
    pub batch_num: u32,
    pub chain_len: usize,
    /// The timestamp of when the writing peer created the link for this
    /// chain top to the chain batch anchor.
    pub timestamp: Timestamp,
}

#[hdk_extern]
fn get_chain_top_for_batch(input: GetChainTopForBatchInput) -> ExternResult<BatchChainTop> {
    let links: Vec<Link> = get_links(
        LinkQuery::try_new(
            chain_batch_anchor(input.agent.clone(), input.batch_num)?,
            LinkTypes::LatestAction,
        )?,
        GetStrategy::Network,
    )?;
    let batch_chain_top_link: Link = links
        .last()
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(String::from(
                "No batch chain top link found"
            )))
        })?
        .clone();
    let chain_top: ActionHash = batch_chain_top_link.target.try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(String::from(
            "Invalid link target type"
        )))
    })?;
    let serialized_bytes = SerializedBytes::from(UnsafeBytes::from(batch_chain_top_link.tag.0));
    let chain_len_tag: ChainLenTag = serialized_bytes.try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(
            "Failed to deserialize tag to chain len".to_string()
        ))
    })?;
    Ok(BatchChainTop {
        chain_top,
        batch_num: input.batch_num,
        chain_len: chain_len_tag.0,
        timestamp: batch_chain_top_link.timestamp,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SampleEntryInput {
    pub agent: AgentPubKey,
    pub chain_top: ActionHash,
    pub chain_len: usize,
}

#[hdk_extern]
fn create_validated_sample_entry(input: SampleEntryInput) -> ExternResult<()> {
    // Create entry that validates with must_get_agent_activity
    let _ = create_entry(EntryTypes::ValidatedSampleEntry(ValidatedSampleEntry {
        agent: input.agent,
        chain_top: input.chain_top,
        chain_len: input.chain_len,
    }))?;

    Ok(())
}

#[hdk_extern]
fn announce_write_behaviour() -> ExternResult<ActionHash> {
    create_link(
        write_agents_anchor()?,
        agent_info()?.agent_initial_pubkey,
        LinkTypes::AgentBehaviour,
        (),
    )
}

#[hdk_extern]
fn get_random_agent_with_write_behaviour() -> ExternResult<Option<AgentPubKey>> {
    let mut links = get_links(
        LinkQuery::try_new(write_agents_anchor()?, LinkTypes::AgentBehaviour)?,
        GetStrategy::default(),
    )?;
    links.shuffle(&mut rng());
    let agent: Option<AgentPubKey> = links
        .first()
        .map(|l| AgentPubKey::try_from(l.target.clone()))
        .transpose()
        .map_err(|_| {
            wasm_error!(WasmErrorInner::Guest(
                "Failed to cast to AgentPubKey".to_string()
            ))
        })?;
    Ok(agent)
}
