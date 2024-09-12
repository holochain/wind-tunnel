use hdi::prelude::*;

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord, Hash, Serialize, Deserialize, SerializedBytes, Debug)]
#[serde(tag = "type")]
pub enum TimedMessage {
    TimedRequest {
        requester: AgentPubKey,
        responder: AgentPubKey,
        requested_at: Timestamp,
    },
    TimedResponse {
        requester: AgentPubKey,
        responder: AgentPubKey,
        requested_at: Timestamp,
        responded_at: Timestamp,
    },
}

impl TimedMessage {
    pub fn requester(&self) -> &AgentPubKey {
        match self {
            Self::TimedRequest { requester, .. } => requester,
            Self::TimedResponse { requester, .. } => requester,
        }
    }

    pub fn responder(&self) -> &AgentPubKey {
        match self {
            Self::TimedRequest { responder, .. } => responder,
            Self::TimedResponse { responder, .. } => responder,
        }
    }

    pub fn requested_at(&self) -> Timestamp {
        match self {
            Self::TimedRequest { requested_at, .. } => *requested_at,
            Self::TimedResponse { requested_at, .. } => *requested_at,
        }
    }

    pub fn to_response(&self, responded_at: Timestamp) -> TimedMessage {
        Self::TimedResponse {
            requester: self.requester().clone(),
            responder: self.responder().clone(),
            requested_at: self.requested_at(),
            responded_at,
        }
    }

    pub fn to_request(&self) -> TimedMessage {
        Self::TimedRequest {
            requester: self.requester().clone(),
            responder: self.responder().clone(),
            requested_at: self.requested_at(),
        }
    }
}
