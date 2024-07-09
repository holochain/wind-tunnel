use hdi::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ImportantAgreement(ImportantAgreement),
    ParticipantBase(ParticipantBase),
}

#[derive(Clone)]
#[hdk_entry_helper]
pub struct ImportantAgreement {
    pub best_ice_cream_flavour: String,
}

#[hdk_entry_helper]
pub struct ParticipantBase;

#[hdk_link_types]
pub enum LinkTypes {
    Participant,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Signals {
    AcceptedRequest(AcceptedRequest),
    Response(PreflightResponse),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AcceptedRequest {
    pub preflight_request: PreflightRequest,
    pub preflight_response: PreflightResponse,
}
