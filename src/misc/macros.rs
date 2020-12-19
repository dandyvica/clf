/// Auto-implement the FromStr trait for a struct
#[macro_export]
macro_rules! fromstr {
    ($t:ty) => {
        impl std::str::FromStr for $t {
            type Err = serde_yaml::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                serde_yaml::from_str(s)
            }
        }
    };
}
