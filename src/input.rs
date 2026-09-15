use std::io::{IsTerminal, Read};

/// zstd magic bytes; used to auto-detect compressed input.
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// Read a recording from a file path or stdin (`-`), transparently
/// decompressing `.cast.zst` content detected via magic bytes.
pub fn read_cast_text(path: &str) -> anyhow::Result<String> {
    let bytes = if path == "-" {
        if std::io::stdin().is_terminal() {
            anyhow::bail!(
                "refusing to read recording from interactive stdin (use `-` with a pipe or pass a file)"
            );
        }
        let mut buf = Vec::new();
        std::io::stdin().lock().read_to_end(&mut buf)?;
        buf
    } else {
        std::fs::read(path).map_err(|e| anyhow::anyhow!("cannot read {path}: {e}"))?
    };

    if bytes.len() >= 4 && bytes[0..4] == ZSTD_MAGIC {
        let mut decoder = zstd::stream::read::Decoder::new(&bytes[..])?;
        let mut text = String::new();
        decoder.read_to_string(&mut text)?;
        Ok(text)
    } else {
        String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("recording is not valid UTF-8: {e}"))
    }
}
