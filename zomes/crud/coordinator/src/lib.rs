use crud_integrity::{EntryTypes, UnitEntryTypes, SampleEntry};
use hdk::prelude::*;

#[hdk_extern]
fn init() -> ExternResult<InitCallbackResult> {
    create_cap_grant(CapGrantEntry {
        tag: "access".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(BTreeSet::from([(
            "crud".into(),
            "create_sample_entry".into(),
        )])),
    })?;

    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
fn create_sample_entry(value: String) -> ExternResult<ActionHash> {
    create_entry(EntryTypes::SampleEntry(SampleEntry { value }))
}

#[hdk_extern]
fn get_sample_entry(hash: ActionHash) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::local())
}

#[hdk_extern]
fn chain_query_count_len() -> ExternResult<u32> {
    let q = ChainQueryFilter::new()
        .include_entries(true)
        .entry_type(UnitEntryTypes::SampleEntry.try_into()?);
    let results = query(q)?;

    let sum = results.into_iter().filter_map(|r| {
        let se: SampleEntry = r.entry.into_option().unwrap().try_into().ok()?;
        Some(se)
    }).map(|se| se.value.len()).fold(0, |acc, len| acc + len);

    Ok(sum as u32)
}
