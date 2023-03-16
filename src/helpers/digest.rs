use eyre::Result;

struct HashWrapper<'a>(&'a [u8]);

impl std::fmt::Display for HashWrapper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

pub fn hash_data(algorithm: &'static ring::digest::Algorithm, data: &[u8]) -> String {
    hash_to_string(ring::digest::digest(algorithm, data).as_ref())
}

pub fn hash_to_string(hash: &[u8]) -> String {
    format!("{}", HashWrapper(hash))
}

#[derive(Debug, Clone)]
pub struct RequestValidator {
    token: String,
}

impl RequestValidator {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub fn validate(&self, data: &str, signature: &str) -> Result<bool> {
        let key = &ring::hmac::Key::new(ring::hmac::HMAC_SHA256, self.token.as_bytes());
        let local_signature = ring::hmac::sign(key, data.as_bytes());
        Ok(signature == hash_to_string(local_signature.as_ref()))
    }
}

#[cfg(test)]
mod tt {
    use super::*;
    #[test]
    fn sha2() {
        assert_eq!(
            hash_data(
                &ring::digest::SHA256,
                "wiki/Beatmap/Beatmap_collaborations/en.md".as_bytes()
            ),
            "1e2a9df846abee64d66f7f83b0caaa9ea82afef93ab54c5af59a88d0372c83ee"
        );
    }
}
