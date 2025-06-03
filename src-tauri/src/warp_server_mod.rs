use std::convert::Infallible;
use std::io::Read;
use std::sync::Arc;

use flate2::read::GzDecoder;
use rusqlite::{Connection, OptionalExtension};
use tokio::sync::Mutex;
use warp::{Filter, http::Response, Reply};
use warp::fs::{File, dir};

/// Bir byte dizisinin GZIP olup olmadığını kontrol eder
fn is_gzip_bytes(raw: &[u8]) -> bool {
    raw.len() >= 2 && raw[0] == 0x1f && raw[1] == 0x8b
}

/// Warp tile-server'ı başlatan fonksiyon (owned String parametreler)
pub async fn run_warp(mbtiles_path: String, styles_dir: String, fonts_dir: String) {
    // 1) MBTiles bağlantısını aç
    let conn = Connection::open(&mbtiles_path)
        .expect("Warp: MBTiles açılamadı (yanlış yol?)");
    let db = Arc::new(Mutex::new(conn));
    let db_filter = warp::any().map(move || Arc::clone(&db));

    // -----------------------------------
    // 2) TILES ROUTE: "/tiles/:z/:x/:y.pbf"
    // -----------------------------------
    let tiles_route = warp::path("tiles")
        .and(warp::path::param::<u8>())     // z
        .and(warp::path::param::<u32>())    // x
        .and(warp::path::param::<String>()) // y_with_ext
        .and(warp::path::end())
        .and(db_filter.clone())
        .and_then(
            move |z: u8, x: u32, y_with_ext: String, db: Arc<Mutex<Connection>>| {
                async move {
                    // 2a) y param ile ".pbf" suffix'ini kaldır
                    let y_str = y_with_ext.strip_suffix(".pbf").unwrap_or(&y_with_ext);

                    // 2b) y değerini parse et
                    let y_leaflet: u32 = match y_str.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            // Geçersiz y ise minimal PBF dön
                            let empty_pbf: &[u8] = &[0x0A, 0x00];
                            let resp = Response::builder()
                                .status(200)
                                .header("Content-Type", "application/x-protobuf")
                                .header("Access-Control-Allow-Origin", "*")
                                .body(empty_pbf.to_vec())
                                .unwrap();
                            return Ok::<_, Infallible>(resp);
                        }
                    };

                    // 2c) XYZ → TMS dönüşümü
                    let zoom = z as u32;
                    let tms_y = ((1u32 << zoom) - 1).saturating_sub(y_leaflet);

                    // 2d) MBTiles sorgusu: tile_data blob’u al
                    let row_blob: Option<Vec<u8>> = {
                        let conn = db.lock().await;
                        let mut stmt = conn.prepare(
                            "SELECT tile_data FROM tiles WHERE zoom_level = ? AND tile_column = ? AND tile_row = ?",
                        ).unwrap();
                        stmt.query_row(
                            [z as i32, x as i32, tms_y as i32],
                            |r| r.get::<_, Vec<u8>>(0),
                        ).optional().unwrap()
                    };

                    // 2e) Eğer tile varsa, GZIP aç ve raw PBF döndür
                    if let Some(mut blob) = row_blob {
                        if is_gzip_bytes(&blob) {
                            let mut decoder = GzDecoder::new(&blob[..]);
                            let mut buf = Vec::new();
                            if decoder.read_to_end(&mut buf).is_ok() {
                                blob = buf;
                            } else {
                                blob = vec![0x0A, 0x00];
                            }
                        }
                        let resp = Response::builder()
                            .status(200)
                            .header("Content-Type", "application/x-protobuf")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(blob)
                            .unwrap();
                        Ok::<_, Infallible>(resp)
                    } else {
                        // 2f) Tile yoksa minimal PBF döndür
                        let empty_pbf: &[u8] = &[0x0A, 0x00];
                        let resp = Response::builder()
                            .status(200)
                            .header("Content-Type", "application/x-protobuf")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(empty_pbf.to_vec())
                            .unwrap();
                        Ok::<_, Infallible>(resp)
                    }
                }
            },
        );

    // -----------------------------------
    // 3) STYLES ROUTE: "/styles/*"
    // -----------------------------------
    let styles_path_clone = styles_dir.clone();
    let styles_route = warp::path("styles")
        .and(dir(styles_path_clone))
        .and_then(|file: File| async move {
            let reply = file.into_response();
            let with_cors = warp::reply::with_header(reply, "Access-Control-Allow-Origin", "*");
            Ok::<_, Infallible>(with_cors)
        });

    // -----------------------------------
    // 4) FONTS ROUTE: "/fonts/*"
    // -----------------------------------
    let fonts_path_clone = fonts_dir.clone();
    let fonts_route = warp::path("fonts")
        .and(dir(fonts_path_clone))
        .and_then(|file: File| async move {
            let reply = file.into_response();
            let with_cors = warp::reply::with_header(reply, "Access-Control-Allow-Origin", "*");
            Ok::<_, Infallible>(with_cors)
        });

    // -----------------------------------
    // 5) ROTELERİ BİRLEŞTİR
    // -----------------------------------
    let routes = tiles_route
        .or(styles_route)
        .or(fonts_route);

    // -----------------------------------
    // 6) 0.0.0.0:8081’DE WARP’I BAŞLAT
    // -----------------------------------
    let (_addr, server_future) = warp::serve(routes).bind_ephemeral(([0, 0, 0, 0], 8081));
    println!("▶️ Warp tile-server 0.0.0.0:8081 üzerinde başlatıldı.");
    tokio::spawn(server_future);

    // Sonsuza kadar bekle
    futures::future::pending::<()>().await;
}
