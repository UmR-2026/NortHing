//! OfflineSubAgentProfile: hermetic test profile for sub-agent tests.
//!
//! A profile bundles canned LLM responses (per round) and canned tool
//! calls. Callers drive the profile by calling [`OfflineSubAgentProfile::tick`]
//! once per round; the profile returns the next canned response and (if
//! present) the canned tool call for that round. After the last round, the
//! profile returns `OfflineTickOutput::Done` and refuses further ticks.
//!
//! This profile is intentionally *data-only* — it does not implement
//! `LongRunningSkill` (that would force `test-support` to depend on
//! `northhing-agent-dispatch`, which crosses a layer boundary). The
//! integration test in `tests/offline_subagent_profile.rs` shows how a
//! caller wraps the profile into a `LongRunningSkill` impl.
//!
//! Use cases:
//! - CI regression without an LLM provider
//! - Deterministic transcript replay
//! - Snapshot tests of multi-round sub-agent flow
//!
//! Companion: [`super::fixture_loader`] loads `OfflineSubAgentProfile`
//! from JSON fixtures under `tests/fixtures/llm/`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One canned round: either a text response, a tool call, or both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineRound {
    /// Stable id for this round (used in test assertions).
    #[serde(alias = "round_id")]
    pub round_id: String,
    /// Text the sub-agent emits at this round. Required.
    #[serde(alias = "text")]
    pub text: String,
    /// Optional tool call the sub-agent makes at this round.
    /// If present, the runtime invokes the tool; if absent, this round
    /// is a pure text round.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "tool_call")]
    pub tool_call: Option<OfflineToolCall>,
    /// If true, the round is the final round; the profile returns `Done`
    /// after emitting it. If false (default), the profile returns
    /// `Continue` and the runtime drives another round.
    #[serde(default, alias = "is_final")]
    pub is_final: bool,
}

/// A canned tool call: tool name + structured arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineToolCall {
    #[serde(alias = "tool_name")]
    pub tool_name: String,
    #[serde(alias = "arguments")]
    pub arguments: Value,
}

/// Hermetic test profile for a sub-agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineSubAgentProfile {
    /// Stable profile id (used for telemetry / log correlation).
    #[serde(alias = "profile_id")]
    pub profile_id: String,
    /// Agent type the profile emulates (e.g. "function-agent", "deep-research").
    #[serde(alias = "agent_type")]
    pub agent_type: String,
    /// Rounds to drive, in order. Must be non-empty.
    #[serde(alias = "rounds")]
    pub rounds: Vec<OfflineRound>,
}

/// Output of one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfflineTickOutput {
    /// Drive another round; the runtime invokes the (optional) tool and
    /// feeds the tool result into the next `tick` call.
    Continue {
        round_id: String,
        text: String,
        tool_call: Option<OfflineToolCall>,
    },
    /// Sub-agent is finished; `final_text` is the final response.
    Done { round_id: String, final_text: String },
}

/// Why a tick failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfflineTickError {
    /// Caller asked for round N, but the profile has only M < N rounds.
    /// Includes the profile id for diagnostics.
    RoundOutOfRange { profile_id: String, requested: usize, total: usize },
    /// Profile is empty (zero rounds). Every profile must declare at
    /// least one round; an empty profile is a fixture authoring bug.
    EmptyProfile { profile_id: String },
    /// Round is marked `is_final=true` but the profile has more rounds
    /// after it (contradictory fixture).
    PrematureFinal { profile_id: String, round_id: String },
}

impl std::fmt::Display for OfflineTickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoundOutOfRange { profile_id, requested, total } => write!(
                f,
                "profile {} exhausted: requested round {} but only has {} rounds",
                profile_id, requested, total
            ),
            Self::EmptyProfile { profile_id } => {
                write!(f, "profile {} has zero rounds", profile_id)
            }
            Self::PrematureFinal { profile_id, round_id } => write!(
                f,
                "profile {} round {} marked is_final=true but more rounds follow",
                profile_id, round_id
            ),
        }
    }
}

impl std::error::Error for OfflineTickError {}

impl OfflineSubAgentProfile {
    /// Create a profile with the given id and agent type. Use the
    /// `with_round` builder to add rounds.
    pub fn new(profile_id: impl Into<String>, agent_type: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            agent_type: agent_type.into(),
            rounds: Vec::new(),
        }
    }

    /// Builder: add a non-final round. The text is what the sub-agent
    /// emits; `tool_call` is the (optional) tool the sub-agent invokes.
    pub fn with_round(
        mut self,
        round_id: impl Into<String>,
        text: impl Into<String>,
        tool_call: Option<OfflineToolCall>,
    ) -> Self {
        self.rounds.push(OfflineRound {
            round_id: round_id.into(),
            text: text.into(),
            tool_call,
            is_final: false,
        });
        self
    }

    /// Builder: add a final round. The sub-agent terminates after this round.
    pub fn with_final_round(
        mut self,
        round_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        self.rounds.push(OfflineRound {
            round_id: round_id.into(),
            text: text.into(),
            tool_call: None,
            is_final: true,
        });
        self
    }

    /// Total rounds declared by this profile.
    pub fn round_count(&self) -> usize {
        self.rounds.len()
    }

    /// Drive the next round. `round_index` is 0-based.
    ///
    /// * Round N where `is_final=false` → `Continue` with that round's
    ///   text + optional tool call.
    /// * Round N where `is_final=true` and N is the last round → `Done`
    ///   with the final text.
    /// * Round N >= `round_count()` → `Err(RoundOutOfRange)`.
    pub fn tick(&self, round_index: usize) -> Result<OfflineTickOutput, OfflineTickError> {
        if self.rounds.is_empty() {
            return Err(OfflineTickError::EmptyProfile {
                profile_id: self.profile_id.clone(),
            });
        }
        let round = self.rounds.get(round_index).ok_or_else(|| {
            OfflineTickError::RoundOutOfRange {
                profile_id: self.profile_id.clone(),
                requested: round_index,
                total: self.rounds.len(),
            }
        })?;
        if round.is_final && round_index + 1 < self.rounds.len() {
            return Err(OfflineTickError::PrematureFinal {
                profile_id: self.profile_id.clone(),
                round_id: round.round_id.clone(),
            });
        }
        if round.is_final {
            Ok(OfflineTickOutput::Done {
                round_id: round.round_id.clone(),
                final_text: round.text.clone(),
            })
        } else {
            Ok(OfflineTickOutput::Continue {
                round_id: round.round_id.clone(),
                text: round.text.clone(),
                tool_call: round.tool_call.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn single_round_profile_returns_done_on_first_tick() {
        let p = OfflineSubAgentProfile::new("p1", "echo").with_final_round("r0", "hello world");
        let out = p.tick(0).expect("round 0 exists");
        assert_eq!(
            out,
            OfflineTickOutput::Done {
                round_id: "r0".into(),
                final_text: "hello world".into()
            }
        );
    }

    #[test]
    fn multi_round_profile_continues_then_dones() {
        let tool_call = OfflineToolCall {
            tool_name: "echo".into(),
            arguments: json!({"msg": "hi"}),
        };
        let p = OfflineSubAgentProfile::new("p2", "function-agent")
            .with_round("r0", "calling tool", Some(tool_call.clone()))
            .with_round("r1", "got result", None)
            .with_final_round("r2", "goodbye");
        assert_eq!(p.tick(0).unwrap(),
            OfflineTickOutput::Continue {
                round_id: "r0".into(),
                text: "calling tool".into(),
                tool_call: Some(tool_call.clone()),
            });
        assert_eq!(p.tick(1).unwrap(),
            OfflineTickOutput::Continue {
                round_id: "r1".into(),
                text: "got result".into(),
                tool_call: None,
            });
        assert_eq!(p.tick(2).unwrap(),
            OfflineTickOutput::Done {
                round_id: "r2".into(),
                final_text: "goodbye".into(),
            });
    }

    #[test]
    fn out_of_range_returns_error_with_profile_id() {
        let p = OfflineSubAgentProfile::new("p3", "echo").with_final_round("r0", "done");
        let err = p.tick(5).unwrap_err();
        assert_eq!(
            err,
            OfflineTickError::RoundOutOfRange {
                profile_id: "p3".into(),
                requested: 5,
                total: 1
            }
        );
    }

    #[test]
    fn empty_profile_returns_error() {
        let p = OfflineSubAgentProfile::new("p4", "echo");
        let err = p.tick(0).unwrap_err();
        assert_eq!(
            err,
            OfflineTickError::EmptyProfile {
                profile_id: "p4".into()
            }
        );
    }

    #[test]
    fn premature_final_returns_error() {
        // A final round in the middle of a non-final round is a fixture bug.
        let p = OfflineSubAgentProfile::new("p5", "echo")
            .with_final_round("r0", "early exit")
            .with_round("r1", "should not happen", None);
        let err = p.tick(0).unwrap_err();
        assert_eq!(
            err,
            OfflineTickError::PrematureFinal {
                profile_id: "p5".into(),
                round_id: "r0".into()
            }
        );
    }
}
