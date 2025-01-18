use dotenv::dotenv;
fn main() -> std::io::Result<()> {
    prost_build::compile_protos(&["src/types.proto"], &["src/"])?;

    // read the env vars from .env and set them as rustc env vars so the compiler can read them
    // and include them on compile time

    dotenv().ok();
    match std::env::var("WIFI_PASS") {
        Err(_) => panic!("Error Wifi Pass not set! Please add WIFI_PASS to .env"),
        Ok(pass) => {
            println!("cargo:rustc-env=WIFI_PASS={}", pass);
        }
    }
    match std::env::var("WIFI_SSID") {
        Err(_) => panic!("Error Wifi SSID not set! Please add WIFI_SSID to .env"),
        Ok(ssid) => {
            println!("cargo:rustc-env=WIFI_SSID={}", ssid);
        }
    }
    match std::env::var("SERVER_ADDR") {
        Err(_) => panic!("Error SERVER_ADDR not set! Please add SERVER_ADDR to .env"),
        Ok(addr) => {
            println!("cargo:rustc-env=SERVER_ADDR={}", addr);
        }
    }
    println!("cargo:rustc-env=CRATE_CC_NO_DEFAULTS=1");

    embuild::espidf::sysenv::output();
    Ok(())
}
