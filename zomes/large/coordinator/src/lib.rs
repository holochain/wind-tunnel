use hdk::prelude::*;

#[hdk_extern]
fn init() -> ExternResult<InitCallbackResult> {
    some_regex_fn().ok();

    Ok(InitCallbackResult::Pass)
}

fn some_regex_fn() -> anyhow::Result<()> {
    let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$")?;
    let date = "2021-01-01";
    if re.is_match(date) {
        Ok(())
    } else {
        anyhow::bail!("Invalid date")
    }
}
