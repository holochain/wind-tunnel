use hdi::prelude::*;

#[hdk_entry_helper]
pub struct SampleEntry {
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    SampleEntry(SampleEntry),
}

#[hdk_link_types]
pub enum LinkTypes {
    SampleLink,
}

#[hdk_extern]
fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(OpEntry::CreateEntry { .. })
        | FlatOp::StoreRecord(OpRecord::InitZomesComplete { .. } | OpRecord::CreateEntry { .. })
        | FlatOp::RegisterAgentActivity(
            OpActivity::InitZomesComplete { .. } | OpActivity::CreateEntry { .. },
        ) => Ok(ValidateCallbackResult::Valid),
        _ => Ok(ValidateCallbackResult::Invalid(format!(
            "Validation not supported for type: {:?}",
            op
        ))),
    }
}
