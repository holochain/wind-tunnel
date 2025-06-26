use hdi::hash_path::path::root_hash;
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
        FlatOp::RegisterAgentActivity(OpActivity::CreateLink { .. }) => {
            Ok(ValidateCallbackResult::Valid)
        }
        FlatOp::RegisterCreateLink { link_type, .. } => match link_type {
            LinkTypes::Path => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::StoreRecord(OpRecord::CreateLink {
            base_address,
            action,
            ..
        }) => {
            let mut base_address_lookup = base_address.clone();
            loop {
                // Find the root
                if base_address_lookup == root_hash()? {
                    break;
                }
                if base_address_lookup.into_entry_hash().is_none() {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Base address is not valid entry hash".to_string(),
                    ));
                }
                if let Action::CreateLink(prev_create_link) =
                    must_get_action(action.prev_action.clone())?
                        .hashed
                        .as_content()
                {
                    base_address_lookup = prev_create_link.base_address.clone();
                } else {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Cannot get prev_action".to_string(),
                    ));
                }
            }

            Ok(ValidateCallbackResult::Valid)
        }
        _ => Ok(ValidateCallbackResult::Invalid(format!(
            "Validation not supported for type: {:?}",
            op
        ))),
    }
}
