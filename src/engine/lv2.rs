use anyhow::Result;
use log::debug;
use std::ffi::CStr;

use super::{Plugin, Port, PortType, PortDirection};

/// LV2 plugin discovery using lilv
pub struct Lv2World {
    world: *mut lilv_sys::LilvWorld,
}

impl Lv2World {
    /// Create a new LV2 world and discover all plugins
    pub fn new() -> Result<Self> {
        debug!("Initializing LV2 world...");
        
        unsafe {
            let world = lilv_sys::lilv_world_new();
            if world.is_null() {
                return Err(anyhow::anyhow!("Failed to create lilv world"));
            }
            
            // Load all installed LV2 plugins
            lilv_sys::lilv_world_load_all(world);
            
            Ok(Self { world })
        }
    }
    
    /// Get the list of all available plugins
    pub fn list_plugins(&self) -> Vec<Plugin> {
        debug!("Listing LV2 plugins...");
        
        let mut plugins = Vec::new();
        
        unsafe {
            let all_plugins = lilv_sys::lilv_world_get_all_plugins(self.world);
            
            // Iterate over all plugins
            let mut iter = lilv_sys::lilv_plugins_begin(all_plugins);
            while !lilv_sys::lilv_plugins_is_end(all_plugins, iter) {
                let plugin = lilv_sys::lilv_plugins_get(all_plugins, iter);
                
                // Get plugin URI
                let uri_node = lilv_sys::lilv_plugin_get_uri(plugin);
                let uri_cstr = lilv_sys::lilv_node_as_uri(uri_node);
                let id = if !uri_cstr.is_null() {
                    CStr::from_ptr(uri_cstr).to_string_lossy().to_string()
                } else {
                    String::new()
                };
                
                // Get plugin name
                let name_node = lilv_sys::lilv_plugin_get_name(plugin);
                let name = if !name_node.is_null() {
                    let name_cstr = lilv_sys::lilv_node_as_string(name_node);
                    let name_string = if !name_cstr.is_null() {
                        CStr::from_ptr(name_cstr).to_string_lossy().to_string()
                    } else {
                        id.clone()
                    };
                    lilv_sys::lilv_node_free(name_node);
                    name_string
                } else {
                    id.clone()
                };
                
                if !id.is_empty() {
                    let ports = self.get_plugin_ports(plugin);
                    plugins.push(Plugin { id, name, ports });
                }
                
                iter = lilv_sys::lilv_plugins_next(all_plugins, iter);
            }
        }
        
        debug!("Discovered {} LV2 plugins", plugins.len());
        plugins
    }
    
    /// Get the ports of a plugin
    fn get_plugin_ports(&self, plugin: *const lilv_sys::LilvPlugin) -> Vec<Port> {
        let mut ports = Vec::new();
        
        unsafe {
            let num_ports = lilv_sys::lilv_plugin_get_num_ports(plugin);
            
            // Create LilvNode for port class URIs
            let input_class = lilv_sys::lilv_new_uri(
                self.world,
                b"http://lv2plug.in/ns/lv2core#InputPort\0".as_ptr() as *const i8,
            );
            let output_class = lilv_sys::lilv_new_uri(
                self.world,
                b"http://lv2plug.in/ns/lv2core#OutputPort\0".as_ptr() as *const i8,
            );
            let audio_class = lilv_sys::lilv_new_uri(
                self.world,
                b"http://lv2plug.in/ns/lv2core#AudioPort\0".as_ptr() as *const i8,
            );
            let atom_class = lilv_sys::lilv_new_uri(
                self.world,
                b"http://lv2plug.in/ns/ext/atom#AtomPort\0".as_ptr() as *const i8,
            );
            let control_class = lilv_sys::lilv_new_uri(
                self.world,
                b"http://lv2plug.in/ns/lv2core#ControlPort\0".as_ptr() as *const i8,
            );
            
            for i in 0..num_ports {
                let port = lilv_sys::lilv_plugin_get_port_by_index(plugin, i);
                
                // Get port symbol (ID)
                let symbol_node = lilv_sys::lilv_port_get_symbol(plugin, port);
                let symbol_cstr = lilv_sys::lilv_node_as_string(symbol_node);
                let id = if !symbol_cstr.is_null() {
                    CStr::from_ptr(symbol_cstr).to_string_lossy().to_string()
                } else {
                    continue; // Skip ports without symbols
                };
                
                // Determine port direction
                let is_input = lilv_sys::lilv_port_is_a(plugin, port, input_class);
                let is_output = lilv_sys::lilv_port_is_a(plugin, port, output_class);
                let direction = if is_input {
                    PortDirection::Input
                } else if is_output {
                    PortDirection::Output
                } else {
                    continue; // Skip ports that are neither input nor output
                };
                
                // Determine port type
                let is_audio = lilv_sys::lilv_port_is_a(plugin, port, audio_class);
                let is_atom = lilv_sys::lilv_port_is_a(plugin, port, atom_class);
                let is_control = lilv_sys::lilv_port_is_a(plugin, port, control_class);
                
                // Skip control ports, only include audio and atom (MIDI) ports
                if is_control {
                    continue;
                }
                
                let port_type = if is_audio {
                    PortType::Audio
                } else if is_atom {
                    PortType::Midi // AtomPorts are used for MIDI in LV2
                } else {
                    continue; // Skip other port types
                };
                
                ports.push(Port {
                    id,
                    port_type,
                    direction,
                });
            }
            
            // Free the class URIs
            lilv_sys::lilv_node_free(input_class);
            lilv_sys::lilv_node_free(output_class);
            lilv_sys::lilv_node_free(audio_class);
            lilv_sys::lilv_node_free(atom_class);
            lilv_sys::lilv_node_free(control_class);
        }
        
        ports
    }
}

impl Drop for Lv2World {
    fn drop(&mut self) {
        unsafe {
            if !self.world.is_null() {
                lilv_sys::lilv_world_free(self.world);
            }
        }
    }
}

// Ensure Lv2World is safe to send between threads
unsafe impl Send for Lv2World {}
