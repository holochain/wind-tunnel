use hdk::prelude::*;
use agent_activity_integrity::{EntryTypes, SampleEntry};

#[hdk_extern]
fn create_sample_entry(value: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { value }))
}

#[hdk_extern]
fn get_agent_activity_full(agent: AgentPubKey) -> ExternResult<AgentActivity> {
    let activity = get_agent_activity(
        agent,
        ChainQueryFilter::new(),
        ActivityRequest::Full
    )?;

    Ok(activity)
}
