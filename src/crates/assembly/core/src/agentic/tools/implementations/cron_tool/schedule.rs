use crate::service::cron::CronSchedule;
use crate::util::errors::{NortHingError, NortHingResult};
use chrono::{DateTime, Local, SecondsFormat, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CronAction {
    GetTime,
    List,
    Add,
    Update,
    Remove,
    Run,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CronToolScheduleInput {
    At { at: String },
    Every { every: u64, anchor: Option<String> },
    Cron { expr: String, tz: Option<String> },
}

impl CronToolScheduleInput {
    pub fn to_service_schedule(&self, field_name: &str) -> NortHingResult<CronSchedule> {
        match self {
            Self::At { at } => {
                let at = at.trim();
                if at.is_empty() {
                    return Err(NortHingError::tool(format!("{}.at cannot be empty", field_name)));
                }
                parse_iso_timestamp_ms(at, &format!("{}.at", field_name))?;
                Ok(CronSchedule::At { at: at.to_string() })
            }
            Self::Every { every, anchor } => {
                let anchor_ms = match anchor.as_deref() {
                    Some(anchor) if anchor.trim().is_empty() => {
                        return Err(NortHingError::tool(format!(
                            "{}.anchor cannot be empty when provided",
                            field_name
                        )));
                    }
                    Some(anchor) => Some(parse_iso_timestamp_ms(
                        anchor.trim(),
                        &format!("{}.anchor", field_name),
                    )?),
                    None => None,
                };

                Ok(CronSchedule::Every {
                    every_ms: seconds_to_every_ms(*every, field_name)?,
                    anchor_ms,
                })
            }
            Self::Cron { expr, tz } => {
                let expr = expr.trim();
                if expr.is_empty() {
                    return Err(NortHingError::tool(format!("{}.expr cannot be empty", field_name)));
                }

                Ok(CronSchedule::Cron {
                    expr: expr.to_string(),
                    tz: tz
                        .as_ref()
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CronToolScheduleOutput {
    At {
        at: String,
    },
    Every {
        every: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        anchor: Option<String>,
    },
    Cron {
        expr: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tz: Option<String>,
    },
}

impl TryFrom<&CronSchedule> for CronToolScheduleOutput {
    type Error = NortHingError;

    fn try_from(schedule: &CronSchedule) -> NortHingResult<Self> {
        match schedule {
            CronSchedule::At { at } => Ok(Self::At { at: at.clone() }),
            CronSchedule::Every { every_ms, anchor_ms } => Ok(Self::Every {
                every: every_ms_to_seconds(*every_ms),
                anchor: anchor_ms
                    .map(|value| format_iso_timestamp_local(value, "anchor"))
                    .transpose()?,
            }),
            CronSchedule::Cron { expr, tz } => Ok(Self::Cron {
                expr: expr.clone(),
                tz: tz.clone(),
            }),
        }
    }
}

pub fn parse_iso_timestamp_ms(value: &str, field_name: &str) -> NortHingResult<i64> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|error| {
        NortHingError::tool(format!("{} must be a valid ISO-8601 timestamp: {}", field_name, error))
    })?;
    Ok(parsed.timestamp_millis())
}

pub fn format_iso_timestamp_local(timestamp_ms: i64, field_name: &str) -> NortHingResult<String> {
    let datetime = Local
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .ok_or_else(|| NortHingError::tool(format!("{} timestamp is out of range: {}", field_name, timestamp_ms)))?;
    Ok(datetime.to_rfc3339_opts(SecondsFormat::Secs, false))
}

pub fn every_ms_to_seconds(every_ms: u64) -> u64 {
    every_ms.div_ceil(1_000)
}

pub fn seconds_to_every_ms(seconds: u64, field_name: &str) -> NortHingResult<u64> {
    if seconds == 0 {
        return Err(NortHingError::tool(format!(
            "{}.every must be greater than 0 seconds",
            field_name
        )));
    }

    seconds
        .checked_mul(1_000)
        .ok_or_else(|| NortHingError::tool(format!("{}.every is too large", field_name)))
}

pub fn schedule_summary(schedule: &CronSchedule) -> String {
    match schedule {
        CronSchedule::At { at } => format!("at {}", at),
        CronSchedule::Every { every_ms, anchor_ms } => match anchor_ms {
            Some(anchor_ms) => match format_iso_timestamp_local(*anchor_ms, "anchor") {
                Ok(anchor) => format!("every {}s from {}", every_ms_to_seconds(*every_ms), anchor),
                Err(_) => format!("every {}s", every_ms_to_seconds(*every_ms)),
            },
            None => format!("every {}s", every_ms_to_seconds(*every_ms)),
        },
        CronSchedule::Cron { expr, tz } => match tz.as_deref() {
            Some(tz) if !tz.trim().is_empty() => format!("cron {} ({})", expr, tz),
            _ => format!("cron {} (local timezone)", expr),
        },
    }
}
