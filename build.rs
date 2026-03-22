fn main() {
    println!("cargo::rustc-check-cfg=cfg(esp_idf_comp_espressif__esp32_camera_enabled)");
    println!("cargo::rustc-check-cfg=cfg(esp_idf_comp_espressif__esp_sr_enabled)");
    embuild::espidf::sysenv::output();
}
