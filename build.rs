fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_manifest(
            r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="1.0.0.0"
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
        res.set("FileDescription", "MediaForge — All-in-One Media Converter");
        res.set("ProductName", "MediaForge");
        res.set("FileVersion", "1.0.0.0");
        res.set("ProductVersion", "1.0.0.0");
        res.set("LegalCopyright", "MediaForge © 2026");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {e}");
        }
    }
}
