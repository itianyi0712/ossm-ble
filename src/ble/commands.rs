// BLE Command handler for OSSM
// Parses and executes commands received via BLE characteristic

use log::{info, warn};
use crate::context::AppContext;

/// Commands supported by BLE interface
#[derive(Debug, Clone)]
pub enum BleCommand {
    /// Go to a specific mode
    GoTo(String),
    /// Set speed (0-100)
    SetSpeed(i32),
    /// Set stroke (0-100)
    SetStroke(i32),
    /// Set depth (0-100)
    SetDepth(i32),
    /// Set sensation (0-100)
    SetSensation(i32),
    /// Set pattern index
    SetPattern(i32),
    /// Unknown/invalid command
    Unknown(String),
}

impl BleCommand {
    /// Parse a command string into a BleCommand
    pub fn parse(cmd: &str) -> Self {
        let cmd = cmd.trim();
        
        // Handle "go:" commands
        if let Some(mode) = cmd.strip_prefix("go:") {
            return BleCommand::GoTo(mode.to_string());
        }
        
        // Handle "set:" commands
        if let Some(rest) = cmd.strip_prefix("set:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                let param = parts[0];
                if let Ok(value) = parts[1].parse::<i32>() {
                    return match param {
                        "speed" => BleCommand::SetSpeed(value),
                        "stroke" => BleCommand::SetStroke(value),
                        "depth" => BleCommand::SetDepth(value),
                        "sensation" => BleCommand::SetSensation(value),
                        "pattern" => BleCommand::SetPattern(value),
                        _ => BleCommand::Unknown(cmd.to_string()),
                    };
                }
            }
        }
        
        BleCommand::Unknown(cmd.to_string())
    }
}

/// Handle a BLE command and execute it
pub fn handle_ble_command(cmd_str: &str, app_context: &AppContext) -> anyhow::Result<String> {
    let command = BleCommand::parse(cmd_str);
    info!("Parsed BLE command: {:?}", command);
    
    match command {
        BleCommand::GoTo(mode) => {
            handle_goto(&mode, app_context)
        }
        BleCommand::SetSpeed(value) => {
            handle_set_speed(value, app_context)
        }
        BleCommand::SetStroke(value) => {
            handle_set_stroke(value, app_context)
        }
        BleCommand::SetDepth(value) => {
            handle_set_depth(value, app_context)
        }
        BleCommand::SetSensation(value) => {
            handle_set_sensation(value, app_context)
        }
        BleCommand::SetPattern(value) => {
            handle_set_pattern(value, app_context)
        }
        BleCommand::Unknown(cmd) => {
            warn!("Unknown BLE command: {}", cmd);
            Err(anyhow::anyhow!("Unknown command: {}", cmd))
        }
    }
}

fn handle_goto(mode: &str, app_context: &AppContext) -> anyhow::Result<String> {
    let mut controller = app_context.motor_controller.lock().unwrap();
    
    match mode {
        "simplePenetration" | "strokeEngine" => {
            if let Some(mc) = controller.as_mut() {
                let mut config = mc.get_config();
                config.paused = false;
                mc.set_config(config);
                info!("Started motor in mode: {}", mode);
            }
            Ok(format!("ok:go:{}", mode))
        }
        "menu" => {
            if let Some(mc) = controller.as_mut() {
                let mut config = mc.get_config();
                config.paused = true;
                config.bpm = 0.0;
                mc.set_config(config);
                info!("Returned to menu (paused)");
            }
            Ok("ok:go:menu".to_string())
        }
        _ => {
            warn!("Unknown mode: {}", mode);
            Err(anyhow::anyhow!("Unknown mode: {}", mode))
        }
    }
}

fn handle_set_speed(value: i32, app_context: &AppContext) -> anyhow::Result<String> {
    let value = value.clamp(0, 100);
    let mut controller = app_context.motor_controller.lock().unwrap();
    
    if let Some(mc) = controller.as_mut() {
        let mut config = mc.get_config();
        // Map 0-100 to BPM range (0-300)
        config.bpm = (value as f32) * 3.0;
        // If speed > 0, unpause
        if value > 0 {
            config.paused = false;
        }
        mc.set_config(config);
        info!("Set speed to {}% (BPM: {})", value, config.bpm);
        Ok(format!("ok:set:speed:{}", value))
    } else {
        Err(anyhow::anyhow!("Motor controller not available"))
    }
}

fn handle_set_stroke(value: i32, app_context: &AppContext) -> anyhow::Result<String> {
    let value = value.clamp(0, 100);
    let mut controller = app_context.motor_controller.lock().unwrap();
    
    if let Some(mc) = controller.as_mut() {
        let mut config = mc.get_config();
        // Map 0-100 to depth 0.0-1.0
        config.depth = (value as f32) / 100.0;
        mc.set_config(config);
        info!("Set stroke to {}%", value);
        Ok(format!("ok:set:stroke:{}", value))
    } else {
        Err(anyhow::anyhow!("Motor controller not available"))
    }
}

fn handle_set_depth(value: i32, app_context: &AppContext) -> anyhow::Result<String> {
    let value = value.clamp(0, 100);
    let mut controller = app_context.motor_controller.lock().unwrap();
    
    if let Some(mc) = controller.as_mut() {
        let mut config = mc.get_config();
        // depth_top controls direction
        config.depth_top = value < 50;
        config.paused_position = (value as f32) / 100.0;
        mc.set_config(config);
        info!("Set depth to {}%", value);
        Ok(format!("ok:set:depth:{}", value))
    } else {
        Err(anyhow::anyhow!("Motor controller not available"))
    }
}

fn handle_set_sensation(value: i32, app_context: &AppContext) -> anyhow::Result<String> {
    let value = value.clamp(0, 100);
    let mut controller = app_context.motor_controller.lock().unwrap();
    
    if let Some(mc) = controller.as_mut() {
        let mut config = mc.get_config();
        // Map sensation to sharpness (0-100 -> 0.01-0.99)
        config.sharpness = 0.01 + (value as f32) * 0.0098;
        mc.set_config(config);
        info!("Set sensation to {}%", value);
        Ok(format!("ok:set:sensation:{}", value))
    } else {
        Err(anyhow::anyhow!("Motor controller not available"))
    }
}

fn handle_set_pattern(value: i32, app_context: &AppContext) -> anyhow::Result<String> {
    let pattern_index = value.clamp(0, 6);
    let mut controller = app_context.motor_controller.lock().unwrap();
    
    if let Some(mc) = controller.as_mut() {
        let mut config = mc.get_config();
        
        // Map pattern index to wave function and parameters
        match pattern_index {
            0 => { // Simple Stroke
                config.wave_func = "sine".to_string();
            }
            1 => { // Teasing
                config.wave_func = "sine".to_string();
                config.depth = 0.3;
            }
            2 => { // Robotic
                config.wave_func = "thrust".to_string();
                config.sharpness = 0.1;
            }
            3 => { // Half'n Half
                config.wave_func = "thrust".to_string();
                config.sharpness = 0.5;
            }
            4 => { // Deeper
                config.wave_func = "sine".to_string();
                config.depth = 1.0;
                config.depth_top = false;
            }
            5 => { // Stop'n Go
                config.wave_func = "thrust".to_string();
                config.sharpness = 0.9;
            }
            6 => { // Insist
                config.wave_func = "thrust".to_string();
                config.sharpness = 0.2;
                config.depth = 0.8;
            }
            _ => {}
        }
        
        mc.set_config(config)?;
        info!("Set pattern to index {}", pattern_index);
        Ok(format!("ok:set:pattern:{}", pattern_index))
    } else {
        Err(anyhow::anyhow!("Motor controller not available"))
    }
}
