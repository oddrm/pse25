#![allow(unused)]

use std::path::PathBuf;

use tokio::sync::mpsc::Sender;

use crate::{
    error::Error, plugin_manager::plugin::Plugin, storage::storage_manager::StorageManager,
};

#[derive(Debug, Clone)]
pub struct PluginManager {}

// TODO use PluginError later, now too much refactoring needed
impl PluginManager {
    pub fn new(event_transmitter: StorageManager) -> Self {
        todo!()
    }

    pub fn register_plugins(&mut self, directory: PathBuf) -> Result<(), Error> {
        todo!()
    }

    pub fn register_plugin(&mut self, path: PathBuf) -> Result<(), Error> {
        todo!()
    }

    pub fn start_plugin_instance(
        &mut self,
        plugin: &Plugin,
        parameters: Vec<(String, String)>,
        temp_directory: PathBuf,
        instance_id: InstanceID,
    ) -> Result<(), Error> {
        todo!()
    }

    pub fn stop_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        todo!()
    }

    pub fn pause_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        todo!()
    }

    pub fn resume_plugin_instance(&mut self, instance_id: InstanceID) -> Result<(), Error> {
        todo!()
    }

    pub fn get_running_instances(&self) -> Vec<(&Plugin, InstanceID)> {
        todo!()
    }

    pub fn get_registered_plugins(&self) -> Vec<&Plugin> {
        todo!()
    }
}

type InstanceID = u64;
