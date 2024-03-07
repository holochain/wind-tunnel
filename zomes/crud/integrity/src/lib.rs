use hdi::prelude::*;

#[hdk_entry_helper]
pub struct SampleEntry {
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_defs]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    SampleEntry(SampleEntry),
}
