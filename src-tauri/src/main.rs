// Evita que se abra una consola negra junto a la ventana en Windows (release).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    free_pos_lib::run();
}
