use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Indicates the game this FMG is for, and thus the format it will be written in
pub enum Version {
    /// Demon's Souls
    DemonsSouls = 0,
    /// Dark Souls 1 and Dark Souls 2
    DarkSouls1 = 1,
    /// Bloodborne and Dark Souls 3
    DarkSouls3 = 2,
}

impl TryFrom<u8> for Version {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::DemonsSouls),
            1 => Ok(Self::DarkSouls1),
            2 => Ok(Self::DarkSouls3),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid enum value",
            )),
        }
    }
}