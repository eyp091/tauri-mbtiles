mod warp_server_mod;

use tauri::{App, Manager};
use std::env;
use std::path::PathBuf;

fn main() {
  tauri::Builder::default()
    .setup(|app: &mut App<_>| {
      let is_dev = cfg!(debug_assertions);

      let (styles_dir, fonts_dir) = if is_dev {
        // Dev mod: proje kökünden “src/styles” ve “src/fonts” yollarını al
        let mut proj_root = PathBuf::from(env::current_dir().unwrap());
        proj_root.pop(); // şimdi proje kök dizininde (tauri-map/)
        let styles = proj_root.join("src").join("styles");
        let fonts  = proj_root.join("src").join("fonts");
        (styles, fonts)
      } else {
        // Prod mod: resource_dir/dist altındaki dizinleri al
        let resource_dir = app
          .path()
          .resource_dir()
          .expect("resource dir bulunamadı");
        let dist_dir = resource_dir.join("dist");
        let styles = dist_dir.join("styles");
        let fonts  = dist_dir.join("fonts");
        (styles, fonts)
      };

      // MBTiles yolunu belirle
      let mbtiles_path = if is_dev {
        let mut proj_root = PathBuf::from(env::current_dir().unwrap());
        proj_root.pop(); // proje kökü
        proj_root.join("tr12.mbtiles")
      } else {
        let resource_dir = app
          .path()
          .resource_dir()
          .expect("resource dir bulunamadı");
        resource_dir.join("tr12.mbtiles")
      };

      // String’e çevir
      let mbtiles_str = mbtiles_path.to_string_lossy().to_string();
      let styles_str  = styles_dir.to_string_lossy().to_string();
      let fonts_str   = fonts_dir.to_string_lossy().to_string();

      // Yolların varlığını kontrol edin
      if !mbtiles_path.exists() {
        panic!("tr12.mbtiles bulunamadı: {}", mbtiles_path.display());
      }
      if !styles_dir.exists() {
        panic!("styles klasörü bulunamadı: {}", styles_dir.display());
      }
      if !fonts_dir.exists() {
        panic!("fonts klasörü bulunamadı: {}", fonts_dir.display());
      }

      // Warp tile-server’ı arka planda başlat
      tauri::async_runtime::spawn(async move {
        warp_server_mod::run_warp(mbtiles_str, styles_str, fonts_str).await;
      });

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("Tauri çalıştırılamadı");
}
