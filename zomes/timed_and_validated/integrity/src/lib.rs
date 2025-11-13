use hdi::prelude::{hash_type::AnyLinkable, *};

#[hdk_entry_helper]
pub struct TimedSampleEntry {
    pub created_at: Timestamp,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    TimedSampleEntry(TimedSampleEntry),
}

#[hdk_link_types]
pub enum LinkTypes {
    FixedToTimedEntry,
}

macro_rules! handle_error {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                return Ok(ValidateCallbackResult::Invalid(format!(
                    "Validation logic error: ${e:?}"
                )))
            }
        }
    };
}

#[hdk_extern]
fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened()? {
        FlatOp::StoreEntry(OpEntry::CreateEntry { app_entry, .. }) => match app_entry {
            EntryTypes::TimedSampleEntry(entry) => {
                if entry.value.len() > 10 {
                    Ok(ValidateCallbackResult::Valid)
                } else {
                    Ok(ValidateCallbackResult::Invalid(
                        "Value must be longer than 10 characters".to_string(),
                    ))
                }
            }
        },
        FlatOp::StoreEntry(OpEntry::UpdateEntry { app_entry, .. }) => match app_entry {
            EntryTypes::TimedSampleEntry(entry) => {
                if entry.value.len() > 15 && &entry.value[0..7] == "update:" {
                    Ok(ValidateCallbackResult::Valid)
                } else {
                    Ok(ValidateCallbackResult::Invalid(
                        "Value must be longer than 10 characters".to_string(),
                    ))
                }
            }
        },
        FlatOp::RegisterCreateLink {
            link_type,
            action,
            base_address,
            target_address,
            ..
        } => {
            match link_type {
                LinkTypes::FixedToTimedEntry => {
                    if base_address != fixed_base() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Links must point away from fixed base".to_string(),
                        ));
                    }

                    let target = must_get_valid_record(handle_error!(target_address.try_into()))?;

                    let sample_entry_type: AppEntryDef =
                        handle_error!(UnitEntryTypes::TimedSampleEntry.try_into());
                    match target.action() {
                        Action::Create(create)
                            if create.entry_type == EntryType::App(sample_entry_type.clone()) =>
                        {
                            // Okay, base should be a create, sample entry
                        }
                        _ => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "Target must be a create, sample entry".to_string(),
                            ));
                        }
                    }

                    if &action.author != target.action().author() {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Can only create links to your own entries".to_string(),
                        ));
                    }

                    Ok(ValidateCallbackResult::Valid)
                }
            }
        }
        _ => {
            // Allow any other operations
            Ok(ValidateCallbackResult::Valid)
        }
    }
}

pub fn fixed_base() -> AnyLinkableHash {
    let mut result = blake2b_256("fixed".as_bytes());
    for _ in 0..4 {
        result.insert(0, 0);
    }
    AnyLinkableHash::from_raw_36_and_type(result, AnyLinkable::External)
}
