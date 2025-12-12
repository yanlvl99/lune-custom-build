//! Build script for embedding Windows icon.

use std::io;

fn main() -> io::Result<()> {
    // Só roda no Windows
    #[cfg(windows)]
    {
        use std::path::Path;

        // 1. Defina o caminho corretamente (ajuste conforme a estrutura das suas pastas)
        // Dica: Use o caminho relativo à pasta onde está este build.rs
        let icon_path = "../../assets/logo/no-tilt.ico";

        // 2. Verifica se o arquivo existe antes de tentar compilar (Evita o erro chato)
        if Path::new(icon_path).exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon_path);
            res.set("ProductName", "Lune Custom Build");
            res.set("FileDescription", "A standalone Luau runtime");
            res.set("LegalCopyright", "MPL-2.0");

            // 3. Avisa ao Cargo para monitorar esse arquivo
            // Se você mudar o ícone, o Rust recompila automaticamente.
            println!("cargo:rerun-if-changed={}", icon_path);

            // 4. Compila (o ? joga o erro pra fora se falhar, sem precisar do if let)
            res.compile()?;
        } else {
            // Opcional: Avisa no log que não achou, mas não quebra o build
            println!("cargo:warning=Icon not found at: {}", icon_path);
        }
    }

    Ok(())
}