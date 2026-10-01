use std::io::Read;

/// A reader that transparently skips a UTF-8 BOM (or a UTF-16 BOM, by dropping
/// the mark and leaving the rest to fail loudly).
///
/// Not a nicety: Windows tooling writes BOMs by default and `serde_json` rejects
/// them outright, so a `stats.json` or `*.map` saved from PowerShell or an older
/// editor would fail with a baffling "expected value at line 1 column 1". A byte
/// order mark is not JSON.
pub struct BomSkip<R> {
    inner: R,
    /// Bytes already taken from `inner` but not yet handed to the caller.
    prefix: Vec<u8>,
    checked: bool,
}

impl<R: Read> BomSkip<R> {
    pub fn new(inner: R) -> Self {
        Self { inner, prefix: Vec::new(), checked: false }
    }

    fn check(&mut self) -> std::io::Result<()> {
        if self.checked {
            return Ok(());
        }
        self.checked = true;

        let mut probe = [0u8; 3];
        let mut filled = 0usize;
        while filled < probe.len() {
            match self.inner.read(&mut probe[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        let skip = if filled >= 3 && probe[..3] == [0xEF, 0xBB, 0xBF] {
            3
        } else if filled >= 2 && probe[..2] == [0xFE, 0xFF] {
            2
        } else {
            0
        };
        self.prefix.extend_from_slice(&probe[skip..filled]);
        Ok(())
    }
}

impl<R: Read> Read for BomSkip<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.check()?;
        if !self.prefix.is_empty() {
            let n = buf.len().min(self.prefix.len());
            buf[..n].copy_from_slice(&self.prefix[..n]);
            self.prefix.drain(..n);
            return Ok(n);
        }
        self.inner.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::BomSkip;
    use std::io::Read;

    #[test]
    fn bom_is_skipped_and_the_rest_is_intact() {
        let mut with_bom = vec![0xEF, 0xBB, 0xBF];
        with_bom.extend_from_slice(br#"{"a":1}"#);
        let mut r = BomSkip::new(&with_bom[..]);
        let mut out = String::new();
        r.read_to_string(&mut out).unwrap();
        assert_eq!(out, r#"{"a":1}"#);
    }

    #[test]
    fn plain_json_is_untouched() {
        let data = br#"{"a":1}"#;
        let mut r = BomSkip::new(&data[..]);
        let mut out = String::new();
        r.read_to_string(&mut out).unwrap();
        assert_eq!(out, r#"{"a":1}"#);
    }

    #[test]
    fn bom_split_across_reads_is_still_caught() {
        // A reader that hands out one byte at a time is the worst case for BOM
        // detection: the probe loop must keep reading until it has three bytes.
        struct Trickle(Vec<u8>);
        impl Read for Trickle {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.0.is_empty() || buf.is_empty() {
                    return Ok(0);
                }
                buf[0] = self.0.remove(0);
                Ok(1)
            }
        }
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"hello");
        let mut r = BomSkip::new(Trickle(bytes));
        let mut out = String::new();
        r.read_to_string(&mut out).unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn short_files_do_not_panic() {
        for data in [&b""[..], &b"{"[..], &b"{}"[..]] {
            let mut r = BomSkip::new(data);
            let mut out = Vec::new();
            let _ = r.read_to_end(&mut out);
        }
    }
}
