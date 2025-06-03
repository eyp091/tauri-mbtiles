#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod warp_server_mod;

use tauri::App;
use std::env;

fn main() {
    tauri::Builder::default()
        .setup(|_app: &mut App<_>| {
            // Uygulamanın çalıştığı dizini bul
            let exe_dir = env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .expect("Çalışma dizini bulunamadı");

            // assets dizinini ayarla
            let assets_dir = exe_dir.join("_up_").join("src");
            let styles_dir = assets_dir.join("styles");
            let fonts_dir  = assets_dir.join("fonts");
            let mbtiles_path = assets_dir.join("tr12.mbtiles");

            // Dosyaların varlığını kontrol et
            if !mbtiles_path.exists() {
                panic!("tr12.mbtiles bulunamadı: {}", mbtiles_path.display());
            }
            if !styles_dir.exists() {
                panic!("styles dizini bulunamadı: {}", styles_dir.display());
            }
            if !fonts_dir.exists() {
                panic!("fonts dizini bulunamadı: {}", fonts_dir.display());
            }

            // String’e çevir
            let mbtiles_str = mbtiles_path.to_string_lossy().to_string();
            let styles_str  = styles_dir.to_string_lossy().to_string();
            let fonts_str   = fonts_dir.to_string_lossy().to_string();

            // Warp tile-server’ı arka planda başlat
            tauri::async_runtime::spawn(async move {
                warp_server_mod::run_warp(mbtiles_str, styles_str, fonts_str).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri çalıştırılamadı");
}
