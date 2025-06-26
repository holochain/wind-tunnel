use hdi::prelude::*;

#[hdk_entry_helper]
pub struct SampleEntry {
    pub updated: bool,
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
    Path,
}

#[hdk_extern]
fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(OpEntry::CreateEntry { .. })
        | FlatOp::StoreRecord(OpRecord::InitZomesComplete { .. } | OpRecord::CreateEntry { .. })
        | FlatOp::RegisterAgentActivity(
            OpActivity::InitZomesComplete { .. } | OpActivity::CreateEntry { .. },
        ) => Ok(ValidateCallbackResult::Valid),
        FlatOp::StoreRecord(OpRecord::CreateLink { .. }) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterAgentActivity(OpActivity::CreateLink { .. }) => {
            Ok(ValidateCallbackResult::Valid)
        }
        FlatOp::RegisterCreateLink { link_type, .. } => match link_type {
            LinkTypes::Path => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Invalid(format!(
            "Validation not supported for type: {:?}",
            op
        ))),
    }
}
