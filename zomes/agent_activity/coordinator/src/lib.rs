use agent_activity_integrity::{EntryTypes, LinkTypes, SampleEntry};
use hdk::prelude::*;
use rand::prelude::SliceRandom;
use rand::thread_rng;

fn write_agents_anchor() -> ExternResult<EntryHash> {
    Path::from("WRITE_AGENTS").path_entry_hash()
}

#[hdk_extern]
fn create_sample_entry(value: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { value }))
}

#[hdk_extern]
fn get_agent_activity_full(agent: AgentPubKey) -> ExternResult<AgentActivity> {
    get_agent_activity(agent, ChainQueryFilter::new(), ActivityRequest::Full)
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
        GetLinksInputBuilder::try_new(write_agents_anchor()?, LinkTypes::AgentBehaviour)?.build(),
    )?;
    links.shuffle(&mut thread_rng());
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
