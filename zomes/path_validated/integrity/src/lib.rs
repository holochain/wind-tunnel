use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(PartialEq, Eq)]
pub struct BookEntry {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    BookEntry(BookEntry),
}

#[hdk_link_types]
pub enum LinkTypes {
    AuthorPath,
    AuthorBook,
}

#[hdk_extern]
fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreRecord(OpRecord::CreateLink {
            link_type: LinkTypes::AuthorPath,
            tag,
            ..
        }) => {
            let tag_bytes = SerializedBytes::from(UnsafeBytes::from(tag.into_inner()));
            let tag_component = Component::try_from(tag_bytes).map_err(|e| wasm_error!(e))?;
            let tag_string = String::try_from(&tag_component).map_err(|e| wasm_error!(e))?;

            if tag_string
                .chars()
                .all(|c| c == '-' || c.is_ascii_lowercase())
            {
                Ok(ValidateCallbackResult::Valid)
            } else {
                Ok(ValidateCallbackResult::Invalid(format!(
                    "Link's tag of '{tag_string:?}' contained more than lower-case ASCII letters and dashes",
                )))
            }
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
