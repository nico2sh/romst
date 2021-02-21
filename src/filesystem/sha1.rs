use sha1::{Digest, Sha1};

pub struct SHA1Hasher {
    hasher: Sha1,
}

impl SHA1Hasher {
    pub fn new() -> Self { Self { hasher: <Sha1 as md5::Digest>::new() } }
    pub fn get_hash(&mut self, data: &[u8]) -> String {
        self.hasher.update(data);
        let hash = self.hasher.finalize_reset();
        format!("{:x}", hash)
    }
}
