use hdi::prelude::*;

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
pub struct TimedRequest {
    pub value: Timestamp,
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug)]
pub struct TimedResponse {
    pub request_value: Timestamp,
    pub value: Timestamp,
}
