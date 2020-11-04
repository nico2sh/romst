#[macro_export]
macro_rules! err {
    ($error:expr) => [{
        Result::Err(anyhow::anyhow!($error))
    }]
}