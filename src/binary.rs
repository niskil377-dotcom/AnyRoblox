use anyhow::{ensure, Context, Result};

pub struct Reader<'a>(pub &'a [u8]);

impl<'a> Reader<'a> {
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(n).context("Truncated file")?;
        self.0 = tail;
        Ok(head)
    }
    pub fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    pub fn float(&mut self) -> Result<f32> {
        let v = f32::from_bits(self.word()?);
        ensure!(v.is_finite(), "Non-finite coordinate or resolution");
        Ok(v)
    }
    pub fn string(&mut self) -> Result<String> {
        let n = self
            .0
            .iter()
            .take(4096)
            .position(|b| *b == 0)
            .context("Unterminated string")?;
        let s = std::str::from_utf8(self.take(n)?)?.to_owned();
        self.take(1)?;
        Ok(s)
    }
}
