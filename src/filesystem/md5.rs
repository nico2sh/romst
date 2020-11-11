use md5::{Digest, Md5};

pub struct MD5Hasher {
    hasher: Md5,
}

impl MD5Hasher {
    pub fn new() -> Self { Self { hasher: <Md5 as md5::Digest>::new() } }
    pub fn get_hash(&mut self, data: &Vec<u8>) -> String {
        self.hasher.update(data);
        let hash = self.hasher.finalize_reset();
        format!("{:x}", hash)
    }
}

