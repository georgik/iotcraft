// Console command types that are only available when console feature is enabled

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct BlinkCommand {
    pub action: String,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct MqttCommand {
    pub action: String,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct SpawnCommand {
    pub device_id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct SpawnDoorCommand {
    pub device_id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct PlaceBlockCommand {
    pub block_type: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct RemoveBlockCommand {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct WallCommand {
    pub block_type: String,
    pub x1: i32,
    pub y1: i32,
    pub z1: i32,
    pub x2: i32,
    pub y2: i32,
    pub z2: i32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct SaveMapCommand {
    pub filename: String,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct LoadMapCommand {
    pub filename: String,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct GiveCommand {
    pub item_type: String,
    pub count: u32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct TestErrorCommand {
    pub message: String,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct TeleportCommand {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct LookCommand {
    pub yaw: f32,
    pub pitch: f32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct MoveCommand {
    pub device_id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(feature = "console")]
#[derive(Clone, Debug)]
pub struct ListCommand {}
