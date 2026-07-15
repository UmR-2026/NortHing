use crate::agentic::session::SessionManager;
use crate::agentic::session::{
    CachedSystemPrompt, CachedUserContext, PromptCacheLookup, PromptCacheScope, SessionPromptCache,
    SystemPromptCacheIdentity, UserContextCacheIdentity,
};
use crate::util::errors::NortHingResult;
use tracing::{debug, warn};

impl SessionManager {
    pub(crate) async fn ensure_prompt_cache_loaded(&self, session_id: &str) {
        if self.prompt_cache_store.has_session(session_id) {
            return;
        }

        let cache = if self.should_persist_session_id(session_id) {
            match self.effective_session_workspace_path(session_id).await {
                Some(workspace_path) => {
                    match self
                        .load_prompt_cache_from_persistence(&workspace_path, session_id)
                        .await
                    {
                        Ok(Some(cache)) => cache,
                        Ok(None) => SessionPromptCache::default(),
                        Err(error) => {
                            warn!(
                                "Failed to load prompt cache: session_id={}, workspace_path={}, error={}",
                                session_id,
                                workspace_path.display(),
                                error
                            );
                            SessionPromptCache::default()
                        }
                    }
                }
                None => SessionPromptCache::default(),
            }
        } else {
            SessionPromptCache::default()
        };

        self.prompt_cache_store.replace_cache(session_id, cache);
    }

    pub(crate) async fn cached_system_prompt(
        &self,
        session_id: &str,
        identity: &SystemPromptCacheIdentity,
    ) -> Option<String> {
        self.ensure_prompt_cache_loaded(session_id).await;
        match self.prompt_cache_store.lookup_system_prompt(
            session_id,
            identity,
            self.config.prompt_cache_policy.cache_ttl,
        ) {
            PromptCacheLookup::Hit(prompt) => Some(prompt),
            PromptCacheLookup::Miss => None,
            PromptCacheLookup::Expired => {
                self.persist_prompt_cache_best_effort(session_id, "system_prompt_cache_expired_cleanup")
                    .await;
                None
            }
        }
    }

    pub(crate) async fn remember_system_prompt(
        &self,
        session_id: &str,
        identity: SystemPromptCacheIdentity,
        prompt: String,
    ) {
        self.ensure_prompt_cache_loaded(session_id).await;
        self.prompt_cache_store
            .set_system_prompt(session_id, CachedSystemPrompt::new(identity, prompt));
        self.persist_prompt_cache_best_effort(session_id, "system_prompt_cached")
            .await;
    }

    pub(crate) async fn cached_user_context(
        &self,
        session_id: &str,
        identity: &UserContextCacheIdentity,
    ) -> Option<String> {
        self.ensure_prompt_cache_loaded(session_id).await;
        match self.prompt_cache_store.lookup_user_context(
            session_id,
            identity,
            self.config.prompt_cache_policy.cache_ttl,
        ) {
            PromptCacheLookup::Hit(user_context) => Some(user_context),
            PromptCacheLookup::Miss => None,
            PromptCacheLookup::Expired => {
                self.persist_prompt_cache_best_effort(session_id, "user_context_cache_expired_cleanup")
                    .await;
                None
            }
        }
    }

    pub(crate) async fn remember_user_context(
        &self,
        session_id: &str,
        identity: UserContextCacheIdentity,
        user_context: String,
    ) {
        self.ensure_prompt_cache_loaded(session_id).await;
        self.prompt_cache_store
            .set_user_context(session_id, CachedUserContext::new(identity, user_context));
        self.persist_prompt_cache_best_effort(session_id, "user_context_cached")
            .await;
    }

    pub(crate) async fn clone_prompt_cache(&self, source_session_id: &str, target_session_id: &str) -> bool {
        self.ensure_prompt_cache_loaded(source_session_id).await;
        let Some(cache) = self.prompt_cache_store.get_cache(source_session_id) else {
            return false;
        };
        if cache.is_empty() {
            return false;
        }

        self.prompt_cache_store.replace_cache(target_session_id, cache);
        self.persist_prompt_cache_best_effort(target_session_id, "prompt_cache_cloned")
            .await;
        true
    }

    pub(crate) async fn invalidate_prompt_cache(&self, session_id: &str, scope: PromptCacheScope, reason: &str) {
        self.ensure_prompt_cache_loaded(session_id).await;
        let changed = self.prompt_cache_store.invalidate(session_id, scope);

        if changed {
            debug!(
                "Invalidated session prompt cache: session_id={}, scope={:?}, reason={}",
                session_id, scope, reason
            );
            self.persist_prompt_cache_best_effort(session_id, reason).await;
        }
    }
}
