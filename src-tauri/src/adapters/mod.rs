pub mod current_vehicle_json;
pub mod profile_yaml;

use crate::schema::SchemaAdapter;

pub fn default_adapters() -> Vec<Box<dyn SchemaAdapter>> {
    vec![
        Box::new(current_vehicle_json::CurrentVehicleJsonAdapter),
        Box::new(profile_yaml::ProfileYamlAdapter),
    ]
}
