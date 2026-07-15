//! MiniApp storage port-adapter implementation (split from storage.rs in R38c).
//!
//! Wires `MiniAppStoragePort` callers (port-trait-based storage access) to
//! the concrete `MiniAppStorage` methods that live across `storage.rs`
//! (facade), `storage_app_io.rs`, `storage_drafts.rs`, and
//! `storage_imports_io.rs`. Owns the `MiniAppStorageError` →
//! `MiniAppPortError` translation table at the adapter boundary.

use northhing_product_domains::miniapp::customization::MiniAppCustomizationMetadata;
use northhing_product_domains::miniapp::ports::{
    MiniAppPortError, MiniAppPortErrorKind, MiniAppPortFuture, MiniAppStoragePort,
};

use super::storage::{MiniApp, MiniAppMeta, MiniAppSource, MiniAppStorageError, MiniAppStorageErrorKind};

impl MiniAppStoragePort for super::storage::MiniAppStorage {
    fn list_app_ids(&self) -> MiniAppPortFuture<'_, Vec<String>> {
        Box::pin(async move { self.list_app_ids().await.map_err(map_miniapp_port_error) })
    }

    fn load(&self, app_id: String) -> MiniAppPortFuture<'_, MiniApp> {
        Box::pin(async move { self.load(&app_id).await.map_err(map_miniapp_port_error) })
    }

    fn load_meta(&self, app_id: String) -> MiniAppPortFuture<'_, MiniAppMeta> {
        Box::pin(async move { self.load_meta(&app_id).await.map_err(map_miniapp_port_error) })
    }

    fn load_source(&self, app_id: String) -> MiniAppPortFuture<'_, MiniAppSource> {
        Box::pin(async move { self.load_source_only(&app_id).await.map_err(map_miniapp_port_error) })
    }

    fn save(&self, app: MiniApp) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async move { self.save(&app).await.map_err(map_miniapp_port_error) })
    }

    fn save_version(&self, app_id: String, version: u32, app: MiniApp) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async move {
            self.save_version(&app_id, version, &app)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn load_app_storage(&self, app_id: String) -> MiniAppPortFuture<'_, serde_json::Value> {
        Box::pin(async move { self.load_app_storage(&app_id).await.map_err(map_miniapp_port_error) })
    }

    fn save_app_storage(&self, app_id: String, key: String, value: serde_json::Value) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async move {
            self.save_app_storage(&app_id, &key, value)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn load_draft_app(&self, app_id: String, draft_id: String) -> MiniAppPortFuture<'_, MiniApp> {
        Box::pin(async move {
            self.load_draft_app(&app_id, &draft_id)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn load_draft_manifest(&self, app_id: String, draft_id: String) -> MiniAppPortFuture<'_, serde_json::Value> {
        Box::pin(async move {
            self.load_draft_manifest(&app_id, &draft_id)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn save_draft(
        &self,
        app_id: String,
        draft_id: String,
        app: MiniApp,
        manifest: serde_json::Value,
    ) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async move {
            self.save_draft(&app_id, &draft_id, &app, &manifest)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn delete_draft(&self, app_id: String, draft_id: String) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async move {
            self.delete_draft(&app_id, &draft_id)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn load_customization_metadata(
        &self,
        app_id: String,
    ) -> MiniAppPortFuture<'_, Option<MiniAppCustomizationMetadata>> {
        Box::pin(async move {
            self.load_customization_metadata(&app_id)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn save_customization_metadata(
        &self,
        app_id: String,
        metadata: MiniAppCustomizationMetadata,
    ) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async move {
            self.save_customization_metadata(&app_id, &metadata)
                .await
                .map_err(map_miniapp_port_error)
        })
    }

    fn delete(&self, app_id: String) -> MiniAppPortFuture<'_, ()> {
        Box::pin(async move { self.delete(&app_id).await.map_err(map_miniapp_port_error) })
    }

    fn list_versions(&self, app_id: String) -> MiniAppPortFuture<'_, Vec<u32>> {
        Box::pin(async move { self.list_versions(&app_id).await.map_err(map_miniapp_port_error) })
    }

    fn load_version(&self, app_id: String, version: u32) -> MiniAppPortFuture<'_, MiniApp> {
        Box::pin(async move {
            self.load_version(&app_id, version)
                .await
                .map_err(map_miniapp_port_error)
        })
    }
}

fn map_miniapp_port_error(error: MiniAppStorageError) -> MiniAppPortError {
    let kind = match error.kind() {
        MiniAppStorageErrorKind::NotFound => MiniAppPortErrorKind::NotFound,
        MiniAppStorageErrorKind::Validation => MiniAppPortErrorKind::InvalidInput,
        MiniAppStorageErrorKind::Deserialization => MiniAppPortErrorKind::Deserialization,
        MiniAppStorageErrorKind::Io => MiniAppPortErrorKind::Io,
        MiniAppStorageErrorKind::Backend => MiniAppPortErrorKind::Backend,
    };
    MiniAppPortError::new(kind, error.to_string())
}
