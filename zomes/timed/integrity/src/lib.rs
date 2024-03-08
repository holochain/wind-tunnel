use hdi::prelude::*;

#[hdk_entry_helper]
pub struct TimedEntry {
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_defs]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    TimedEntry(TimedEntry),
}

#[hdk_link_types]
pub enum LinkTypes {
    FixedToTimedEntry,
}
