// BLE GATT Server implementation for OSSM
// Compatible with official OSSM Bluetooth APP protocol

use std::sync::{Arc, Mutex};
use esp32_nimble::{
    enums::*,
    utilities::BleUuid,
    BLEDevice, BLEServer, NimbleProperties,
};
use log::{info, warn};

use crate::context::AppContext;
use super::characteristics::{
    SERVICE_UUID, 
    COMMAND_CHAR_UUID, 
    STATE_CHAR_UUID,
    PATTERNS_CHAR_UUID,
};
use super::commands::handle_ble_command;

/// BLE Server wrapper for OSSM
pub struct BleServer {
    device: &'static BLEDevice,
    app_context: AppContext,
    is_connected: Arc<Mutex<bool>>,
    disconnect_time: Arc<Mutex<Option<std::time::Instant>>>,
}

impl BleServer {
    /// Create a new BLE server instance
    pub fn new(app_context: AppContext) -> Self {
        let device = BLEDevice::take();
        
        Self {
            device,
            app_context,
            is_connected: Arc::new(Mutex::new(false)),
            disconnect_time: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Initialize and start the BLE server
    pub fn start(&self) -> anyhow::Result<()> {
        info!("Initializing BLE server...");
        
        // Set device name
        self.device.set_device_name("OSSM")?;
        
        // Set security
        self.device
            .security()
            .set_auth(AuthReq::SC)
            .set_io_cap(SecurityIOCap::NoInputNoOutput);
        
        let server = self.device.get_server();
        
        // Setup connection callbacks
        let is_connected = self.is_connected.clone();
        let disconnect_time = self.disconnect_time.clone();
        let app_context_disconnect = self.app_context.clone();
        
        server.on_connect(move |server, desc| {
            info!("BLE client connected: {:?}", desc.address());
            *is_connected.lock().unwrap() = true;
            *disconnect_time.lock().unwrap() = None;
            
            // Update connection parameters for better performance
            server.update_conn_params(desc.conn_handle(), 24, 48, 0, 60)
                .ok();
        });
        
        let is_connected_dc = self.is_connected.clone();
        let disconnect_time_dc = self.disconnect_time.clone();
        
        server.on_disconnect(move |_desc, reason| {
            info!("BLE client disconnected, reason: {:?}", reason);
            *is_connected_dc.lock().unwrap() = false;
            *disconnect_time_dc.lock().unwrap() = Some(std::time::Instant::now());
            
            // Start speed ramp-down on disconnect
            if let Some(controller) = app_context_disconnect.motor_controller.lock().unwrap().as_mut() {
                let current_speed = controller.get_config().bpm;
                if current_speed > 0.0 {
                    info!("Starting safety ramp-down from BPM: {}", current_speed);
                    // The main loop will handle gradual ramp-down
                }
            }
        });
        
        // Create OSSM service
        let service = server.create_service(BleUuid::from_uuid128_string(SERVICE_UUID)?);
        
        // Command characteristic (writable)
        let app_context_cmd = self.app_context.clone();
        let command_char = service.lock().create_characteristic(
            BleUuid::from_uuid128_string(COMMAND_CHAR_UUID)?,
            NimbleProperties::READ | NimbleProperties::WRITE,
        );
        
        command_char.lock().on_write(move |args| {
            let data = args.recv_data();
            if let Ok(cmd_str) = std::str::from_utf8(data) {
                info!("BLE command received: {}", cmd_str);
                match handle_ble_command(cmd_str, &app_context_cmd) {
                    Ok(response) => {
                        info!("Command processed: {}", response);
                    }
                    Err(e) => {
                        warn!("Command failed: {}", e);
                    }
                }
            }
        });
        
        // State characteristic (readable, notifiable)
        let app_context_state = self.app_context.clone();
        let state_char = service.lock().create_characteristic(
            BleUuid::from_uuid128_string(STATE_CHAR_UUID)?,
            NimbleProperties::READ | NimbleProperties::NOTIFY,
        );
        
        state_char.lock().on_read(move |_| {
            let state_json = get_current_state_json(&app_context_state);
            info!("BLE state read: {}", state_json);
        });
        
        // Patterns characteristic (readable)
        let patterns_char = service.lock().create_characteristic(
            BleUuid::from_uuid128_string(PATTERNS_CHAR_UUID)?,
            NimbleProperties::READ,
        );
        
        patterns_char.lock().set_value(get_patterns_json().as_bytes());
        
        // Start advertising
        let advertising = self.device.get_advertising();
        advertising.name("OSSM");
        advertising.add_service_uuid(BleUuid::from_uuid128_string(SERVICE_UUID)?);
        advertising.start()?;
        
        info!("BLE server started, advertising as 'OSSM'");
        
        // Start state notification task
        let state_char_notify = state_char.clone();
        let app_context_notify = self.app_context.clone();
        let is_connected_notify = self.is_connected.clone();
        
        std::thread::spawn(move || {
            let mut last_state = String::new();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                if !*is_connected_notify.lock().unwrap() {
                    continue;
                }
                
                let current_state = get_current_state_json(&app_context_notify);
                if current_state != last_state {
                    state_char_notify.lock().set_value(current_state.as_bytes());
                    state_char_notify.lock().notify();
                    last_state = current_state;
                }
            }
        });
        
        Ok(())
    }
    
    /// Check if a client is currently connected
    pub fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }
    
    /// Get time since last disconnect (for ramp-down logic)
    pub fn get_disconnect_elapsed(&self) -> Option<std::time::Duration> {
        self.disconnect_time.lock().unwrap()
            .map(|t| t.elapsed())
    }
}

/// Generate current state as JSON string
fn get_current_state_json(app_context: &AppContext) -> String {
    let (state, speed, stroke, depth, sensation, pattern) = {
        if let Some(controller) = app_context.motor_controller.lock().unwrap().as_ref() {
            let config = controller.get_config();
            let paused = config.paused;
            let state = if paused { 
                "menu.idle" 
            } else { 
                "simplePenetration.idle" 
            };
            (
                state.to_string(),
                config.bpm as i32,
                (config.depth * 100.0) as i32,
                50, // depth position
                50, // sensation
                0,  // pattern
            )
        } else {
            ("error.idle".to_string(), 0, 0, 0, 0, 0)
        }
    };
    
    format!(
        r#"{{"state":"{}","speed":{},"stroke":{},"sensation":{},"depth":{},"pattern":{}}}"#,
        state, speed, stroke, sensation, depth, pattern
    )
}

/// Generate available patterns as JSON
fn get_patterns_json() -> String {
    r#"["Simple Stroke","Teasing","Robotic","Half'n Half","Deeper","Stop'n Go","Insist"]"#.to_string()
}
