// BLE Characteristic UUIDs compatible with official OSSM APP
// These UUIDs are extracted from the official OSSM firmware

/// OSSM BLE Service UUID
pub const SERVICE_UUID: &str = "522b443a-4f53-534d-0001-420badbabe69";

/// Command characteristic - writable, used to send commands
/// Format: "go:<mode>" or "set:<param>:<value>"
pub const COMMAND_CHAR_UUID: &str = "522b443a-4f53-534d-1000-420badbabe69";

/// Speed knob configuration characteristic
pub const SPEED_KNOB_CONFIG_CHAR_UUID: &str = "522b443a-4f53-534d-1010-420badbabe69";

/// State characteristic - readable and notifiable
/// Returns JSON with current state, speed, stroke, sensation, depth, pattern
pub const STATE_CHAR_UUID: &str = "522b443a-4f53-534d-2000-420badbabe69";

/// Patterns list characteristic - readable
/// Returns JSON array of available pattern names
pub const PATTERNS_CHAR_UUID: &str = "522b443a-4f53-534d-3000-420badbabe69";

/// Pattern data characteristic - readable
/// Returns detailed data for a specific pattern
pub const PATTERN_DATA_CHAR_UUID: &str = "522b443a-4f53-534d-3010-420badbabe69";

/// GPIO control characteristic
pub const GPIO_CHAR_UUID: &str = "522b443a-4f53-534d-4000-420badbabe69";
