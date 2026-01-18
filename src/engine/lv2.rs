use anyhow::Result;
use log::debug;
use std::ffi::CStr;

use super::Plugin;

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
                    plugins.push(Plugin { id, name });
                }
                
                iter = lilv_sys::lilv_plugins_next(all_plugins, iter);
            }
        }
        
        debug!("Discovered {} LV2 plugins", plugins.len());
        plugins
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
