pub mod adapters;
pub mod controller;
pub mod model;
pub mod repository;
pub mod schema;
pub mod tower;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            controller::list_vehicles,
            controller::get_vehicle,
            controller::save_vehicle,
            controller::clone_vehicle,
            controller::disable_vehicle,
            controller::validate_vehicle,
            controller::check_tower_linkage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SUMO Config GUI");
}
