use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UpsampleMethod {
    Spsr,
    Default,
}

impl ToString for UpsampleMethod {
    fn to_string(&self) -> String {
        match self {
            UpsampleMethod::Spsr => "spsr",
            UpsampleMethod::Default => "default",
        }
        .to_string()
    }
}

impl FromStr for UpsampleMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "spsr" => Ok(UpsampleMethod::Spsr),
            "default" => Ok(UpsampleMethod::Default),
            _ => Err(format!("{} is not a valid output format", s)),
        }
    }
}
