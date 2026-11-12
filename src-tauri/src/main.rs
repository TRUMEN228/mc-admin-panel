// Без консольного окна в релизной сборке
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mc_admin_panel_lib::run()
}
