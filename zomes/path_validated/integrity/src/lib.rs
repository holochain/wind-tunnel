use hdi::prelude::*;

#[hdk_entry_helper]
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
    Path,
}
