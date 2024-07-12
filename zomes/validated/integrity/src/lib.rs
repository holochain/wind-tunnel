use hdi::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateSampleEntryInput {
    pub original: ActionHash,
    pub new_value: String,
}

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
            EntryTypes::SampleEntry(entry) => {
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
            EntryTypes::SampleEntry(entry) => {
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
                LinkTypes::SampleLink => {
                    let base = must_get_valid_record(handle_error!(base_address.try_into()))?;
                    let target = must_get_valid_record(handle_error!(target_address.try_into()))?;

                    let sample_entry_type: AppEntryDef =
                        handle_error!(UnitEntryTypes::SampleEntry.try_into());
                    match base.action() {
                        Action::Create(create)
                            if create.entry_type == EntryType::App(sample_entry_type.clone()) =>
                        {
                            // Okay, base should be a create, sample entry
                        }
                        _ => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "Base must be a create, sample entry".to_string(),
                            ));
                        }
                    }

                    match target.action() {
                        Action::Update(update)
                            if update.entry_type == EntryType::App(sample_entry_type) =>
                        {
                            // Okay, target should be an update, sample entry
                        }
                        _ => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "Target must be an update, sample entry".to_string(),
                            ));
                        }
                    }

                    if &action.author != base.action().author()
                        || base.action().author() != target.action().author()
                    {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Can only create links to your own updates".to_string(),
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
