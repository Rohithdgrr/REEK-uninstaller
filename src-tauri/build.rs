fn main() {
  // Embed frontend hash for integrity verification at runtime (Audit 2 §1.4)
  // This is a lightweight placeholder — in CI, replace with actual dist hash
  println!("cargo:rustc-env=FRONTEND_BUILD_HASH={}", "audit2-hardened");
  tauri_build::build()
}
