use bevy::asset::AssetLoader;
use bevy::prelude::*;

// RON Asset (moved from levels.rs)
#[derive(Asset, TypePath, Debug)]
pub struct RonAsset(pub String);

#[derive(Default)]
pub struct RonAssetLoader;

impl AssetLoader for RonAssetLoader {
    type Asset = RonAsset;
    type Settings = ();
    type Error = std::io::Error;

    fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let content = String::from_utf8_lossy(&bytes).to_string();
            Ok(RonAsset(content))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

// Script Asset
#[derive(Asset, TypePath, Debug)]
pub struct ScriptAsset(pub String);

#[derive(Default)]
pub struct ScriptAssetLoader;

impl AssetLoader for ScriptAssetLoader {
    type Asset = ScriptAsset;
    type Settings = ();
    type Error = std::io::Error;

    fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let content = String::from_utf8_lossy(&bytes).to_string();
            Ok(ScriptAsset(content))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["rhai"]
    }
}
