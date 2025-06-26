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
    SampleLink,
    Path,
}

#[hdk_extern]
fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreRecord(OpRecord::CreateLink {
            link_type: LinkTypes::Path,
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
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
