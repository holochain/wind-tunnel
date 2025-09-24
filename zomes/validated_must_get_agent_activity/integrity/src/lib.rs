use hdi::prelude::*;

#[hdk_entry_helper]
pub struct SampleEntry {
    pub value: String,
}

#[hdk_entry_helper]
pub struct ValidatedSampleEntry {
    pub agent: AgentPubKey,
    pub chain_head: ActionHash,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    SampleEntry(SampleEntry),
    ValidatedSampleEntry(ValidatedSampleEntry),
}

#[hdk_link_types]
pub enum LinkTypes {
    LatestAction,
}

#[hdk_extern]
fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(OpEntry::CreateEntry { app_entry:  EntryTypes::ValidatedSampleEntry(entry), .. }) => {
            let _ = must_get_agent_activity(entry.agent, ChainFilter::new(entry.chain_head))?;

            Ok(ValidateCallbackResult::Valid)
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
