use hdk::prelude::*;
use path_validated_integrity::{EntryTypes, SampleEntry};

#[hdk_extern]
fn create_sample_entry(value: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { value }))
}
