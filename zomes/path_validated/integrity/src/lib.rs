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
    match op.flattened()? {
        FlatOp::StoreEntry(OpEntry::CreateEntry { app_entry, .. }) => match app_entry {
            EntryTypes::SampleEntry(_) => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { link_type, .. } => match link_type {
            LinkTypes::SampleLink => Ok(ValidateCallbackResult::Valid),
        },
        _ => {
            // Allow any other operations
            Ok(ValidateCallbackResult::Valid)
        }
    }
}
