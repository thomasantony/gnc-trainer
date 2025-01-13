use bevy::prelude::*;

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct Lander;

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct TargetZone;

#[derive(Component)]
pub struct GridSystem;

#[derive(Component)]
pub struct LevelSpecific;

#[derive(Resource, Default)]
pub struct ResetVisibilityFlag(pub bool);
