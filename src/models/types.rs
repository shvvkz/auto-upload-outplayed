use std::str::FromStr;

#[derive(Debug)]
pub struct MatchDetails {
    pub type_queue: QueueId,
    pub champions_name: String,
    pub role: Role,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub friends: Option<Vec<String>>,
}

#[derive(Debug)]
pub enum Role {
    TOP,
    JUNGLE,
    MID,
    ADC,
    SUPPORT
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TOP" => Ok(Role::TOP),
            "JUNGLE" => Ok(Role::JUNGLE),
            "MID" => Ok(Role::MID),
            "BOTTOM" => Ok(Role::ADC),
            "UTILITY" => Ok(Role::SUPPORT),
            _ => Err("Role not found".to_string())
        }
    }
}

#[derive(Debug)]
pub enum QueueId {
    SoloQ,
    Flex,
    NotInterested
}

impl FromStr for QueueId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "420" => Ok(QueueId::SoloQ),
            "440" => Ok(QueueId::Flex),
            _ => Ok(QueueId::NotInterested)
        }
    }
}