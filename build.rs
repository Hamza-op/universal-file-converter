#[cfg(not(target_os = "windows"))]
fn main() {}

#[cfg(target_os = "windows")]
fn main() {
    let package_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let windows_version = format!("{package_version}.0");
    let mut res = winresource::WindowsResource::new();
    let manifest = format!(
        r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="{windows_version}"
    processorArchitecture="amd64"
    name="MediaForge"
    type="win32"
  />
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <description>MediaForge — All-in-One Media Converter</description>
</assembly>
"#,
    );
    res.set_manifest(&manifest);
    res.set("FileDescription", "MediaForge — All-in-One Media Converter");
    res.set("ProductName", "MediaForge");
    res.set("FileVersion", &windows_version);
    res.set("ProductVersion", &windows_version);
    res.set("LegalCopyright", "MediaForge © 2026");
    if let Err(e) = res.compile() {
        eprintln!("Warning: Failed to compile Windows resource: {e}");
    }
}
