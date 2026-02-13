use std::env;

fn main() {
    // Detect target OS via cargo env var, because cfg(target_os = "windows")
    // checks the HOST OS when running build scripts.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        let mut res = winres::WindowsResource::new();
        // Set the manifest to require administrator privileges
        res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#);
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resource: {}", e);
            // Don't panic, maybe user doesn't have windres installed?
            // But we want it to work for cross-compilation.
            // winres uses `windres` (MinGW) or `rc.exe` (MSVC).
        }
    }
}