use super::types::AgentEntry;
use super::visibility::SubagentVisibilityPolicy;
use super::{AgentRegistrationGuard, AgentRegistry};
use crate::agentic::agents::registry::catalog::builtin_agent_specs;
use crate::agentic::agents::{Agent, AgentCategory, SubAgentSource};
use northhing_agent_runtime::agents as runtime_agents;
use northhing_disposable::DisposalGuard;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

pub(crate) fn default_model_id_for_builtin_agent(agent_type: &str) -> &'static str {
    runtime_agents::default_model_id_for_builtin_agent(agent_type)
}

impl AgentRegistry {
    pub(crate) fn build_builtin_agents() -> HashMap<String, AgentEntry> {
        let mut agents = HashMap::new();

        let register = |agents: &mut HashMap<String, AgentEntry>,
                        agent: Arc<dyn Agent>,
                        category: AgentCategory,
                        subagent_source: Option<SubAgentSource>,
                        visibility_policy: SubagentVisibilityPolicy| {
            let id = agent.id().to_string();
            if agents.contains_key(&id) {
                error!("Agent {} already registered, skip registration", id);
                return;
            }
            agents.insert(
                id,
                AgentEntry {
                    category,
                    subagent_source,
                    agent,
                    visibility_policy,
                    custom_config: None,
                },
            );
        };

        for spec in builtin_agent_specs() {
            let source = if spec.category == AgentCategory::SubAgent {
                Some(SubAgentSource::Builtin)
            } else {
                None
            };
            register(
                &mut agents,
                (spec.factory)(),
                spec.category,
                source,
                spec.visibility_policy,
            );
        }

        agents
    }

    /// Create a new agent registry with built-in agents
    pub fn new() -> Self {
        Self {
            agents: Arc::new(std::sync::RwLock::new(Self::build_builtin_agents())),
            project_subagents: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register a new agent and return an RAII [`AgentRegistrationGuard`].
    ///
    /// When the guard is dropped, the agent is removed from the registry
    /// provided the registry entry still references this exact agent instance.
    pub fn register_agent_guarded(
        &self,
        agent: Arc<dyn Agent>,
        category: AgentCategory,
        subagent_source: Option<SubAgentSource>,
        custom_config: Option<super::types::CustomSubagentConfig>,
    ) -> Option<AgentRegistrationGuard> {
        let id = agent.id().to_string();
        let visibility_policy = SubagentVisibilityPolicy::public();
        let mut map = self.write_agents();
        if map.contains_key(&id) {
            error!("Agent {} already registered, skip registration", id);
            return None;
        }
        map.insert(
            id.clone(),
            AgentEntry {
                category,
                subagent_source,
                agent: agent.clone(),
                visibility_policy,
                custom_config,
            },
        );
        drop(map);

        let weak_agents = Arc::downgrade(&self.agents);
        let agent_clone = agent.clone();
        let id_clone = id.clone();

        let disposal = DisposalGuard::new(Box::new(move || {
            if let Some(agents_lock) = weak_agents.upgrade() {
                let mut map = match agents_lock.write() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        warn!("Agent registry write lock poisoned during guard disposal, recovering");
                        poisoned.into_inner()
                    }
                };
                if let Some(entry) = map.get(&id_clone) {
                    if Arc::ptr_eq(&entry.agent, &agent_clone) {
                        map.remove(&id_clone);
                        debug!("Agent {} unregistered by guard", id_clone);
                    }
                }
            }
        }));

        Some(AgentRegistrationGuard {
            guard: disposal,
            agent_id: id,
            agent,
        })
    }

    /// Register a new agent persistently (compatibility method).
    pub fn register_agent(
        &self,
        agent: Arc<dyn Agent>,
        category: AgentCategory,
        subagent_source: Option<SubAgentSource>,
        custom_config: Option<super::types::CustomSubagentConfig>,
    ) {
        if let Some(guard) = self.register_agent_guarded(agent, category, subagent_source, custom_config) {
            guard.disarm();
        }
    }
}
