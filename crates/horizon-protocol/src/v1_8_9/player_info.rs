//! 1.8.9 `PacketPlayOutPlayerInfo`.
//!
//! One packet id carries five actions (add, update game mode, update latency,
//! update display name, remove), and the per-entry body depends on the action.
//! Later versions split the actions and changed the UUID/property encodings.
use crate::{
    Result,
    buf::{Codec, Reader, Writer},
};

/// The 1.8 action ids carried in `PlayerInfoPacket`.
pub const ADD_PLAYER: i32 = 0;
pub const UPDATE_GAME_MODE: i32 = 1;
pub const UPDATE_LATENCY: i32 = 2;
pub const UPDATE_DISPLAY_NAME: i32 = 3;
pub const REMOVE_PLAYER: i32 = 4;

/// One signed profile property (`textures`, ...) in an add-player entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerInfoProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}
/// One player in a `PlayerInfoPacket`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerInfoEntry {
    pub uuid: u128,
    /// Add-player only.
    pub name: Option<String>,
    /// Add-player only.
    pub properties: Vec<PlayerInfoProperty>,
    /// Add-player and update-game-mode.
    pub game_mode: Option<i32>,
    /// Add-player and update-latency.
    pub latency: Option<i32>,
    /// Add-player and update-display-name.
    pub display_name: Option<String>,
}
/// `PacketPlayOutPlayerInfo`: an action plus one entry per player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerInfoPacket {
    pub action: i32,
    pub entries: Vec<PlayerInfoEntry>,
}

fn optional_string(r: &mut Reader) -> Result<Option<String>> {
    if r.bool()? {
        Ok(Some(r.string(32767)?))
    } else {
        Ok(None)
    }
}
fn write_optional_string(value: &Option<String>, w: &mut Writer) {
    w.bool(value.is_some());
    if let Some(value) = value {
        w.string(value);
    }
}

impl Codec<PlayerInfoPacket> for PlayerInfoPacket {
    fn read(r: &mut Reader) -> Result<PlayerInfoPacket> {
        let action = r.varint()?;
        let count = r.varint()?;
        if count < 0 || count as usize > r.remaining() / 16 {
            return Err(crate::Error::new("invalid player-info count"));
        }
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut entry = PlayerInfoEntry {
                uuid: r.uuid()?,
                ..PlayerInfoEntry::default()
            };
            match action {
                ADD_PLAYER => {
                    entry.name = Some(r.string(16)?);
                    let properties = r.varint()?;
                    if properties < 0 || properties as usize > r.remaining() / 2 {
                        return Err(crate::Error::new("invalid player-info property count"));
                    }
                    for _ in 0..properties {
                        entry.properties.push(PlayerInfoProperty {
                            name: r.string(32767)?,
                            value: r.string(32767)?,
                            signature: optional_string(r)?,
                        });
                    }
                    entry.game_mode = Some(r.varint()?);
                    entry.latency = Some(r.varint()?);
                    entry.display_name = optional_string(r)?;
                }
                UPDATE_GAME_MODE => entry.game_mode = Some(r.varint()?),
                UPDATE_LATENCY => entry.latency = Some(r.varint()?),
                UPDATE_DISPLAY_NAME => entry.display_name = optional_string(r)?,
                REMOVE_PLAYER => {}
                other => {
                    return Err(crate::Error::new(format!(
                        "unknown player-info action {other}"
                    )));
                }
            }
            entries.push(entry);
        }
        Ok(PlayerInfoPacket { action, entries })
    }

    fn write(v: &PlayerInfoPacket, w: &mut Writer) {
        w.varint(v.action);
        w.varint(v.entries.len() as i32);
        for entry in &v.entries {
            w.uuid(entry.uuid);
            match v.action {
                ADD_PLAYER => {
                    w.string(entry.name.as_deref().unwrap_or(""));
                    w.varint(entry.properties.len() as i32);
                    for property in &entry.properties {
                        w.string(&property.name);
                        w.string(&property.value);
                        write_optional_string(&property.signature, w);
                    }
                    w.varint(entry.game_mode.unwrap_or(0));
                    w.varint(entry.latency.unwrap_or(0));
                    write_optional_string(&entry.display_name, w);
                }
                UPDATE_GAME_MODE => w.varint(entry.game_mode.unwrap_or(0)),
                UPDATE_LATENCY => w.varint(entry.latency.unwrap_or(0)),
                UPDATE_DISPLAY_NAME => write_optional_string(&entry.display_name, w),
                REMOVE_PLAYER => {}
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(packet: &PlayerInfoPacket) -> Vec<u8> {
        let mut w = Writer::new();
        <PlayerInfoPacket as Codec<PlayerInfoPacket>>::write(packet, &mut w);
        w.data
    }
    fn decode(bytes: &[u8]) -> Result<PlayerInfoPacket> {
        let mut r = Reader::new(bytes);
        let value = <PlayerInfoPacket as Codec<PlayerInfoPacket>>::read(&mut r)?;
        assert_eq!(r.remaining(), 0);
        Ok(value)
    }

    fn entry(uuid: u128) -> PlayerInfoEntry {
        PlayerInfoEntry {
            uuid,
            ..PlayerInfoEntry::default()
        }
    }

    #[test]
    fn add_player_has_profile_properties_and_optional_display_name() {
        let packet = PlayerInfoPacket {
            action: ADD_PLAYER,
            entries: vec![PlayerInfoEntry {
                uuid: 1,
                name: Some("Notch".into()),
                properties: vec![PlayerInfoProperty {
                    name: "textures".into(),
                    value: "value".into(),
                    signature: Some("sig".into()),
                }],
                game_mode: Some(1),
                latency: Some(50),
                display_name: None,
            }],
        };
        let mut expected = vec![ADD_PLAYER as u8, 0x01];
        expected.extend_from_slice(&[0; 15]);
        expected.push(1);
        expected.extend_from_slice(&[0x05, b'N', b'o', b't', b'c', b'h']);
        expected.extend_from_slice(&[0x01, 0x08]);
        expected.extend_from_slice(b"textures");
        expected.extend_from_slice(&[0x05]);
        expected.extend_from_slice(b"value");
        expected.extend_from_slice(&[0x01, 0x03]);
        expected.extend_from_slice(b"sig");
        expected.extend_from_slice(&[0x01, 0x32, 0x00]);
        assert_eq!(encode(&packet), expected);
        assert_eq!(decode(&expected).unwrap(), packet);
    }

    #[test]
    fn update_and_remove_actions_round_trip() {
        let cases = [
            PlayerInfoPacket {
                action: UPDATE_GAME_MODE,
                entries: vec![PlayerInfoEntry {
                    game_mode: Some(2),
                    ..entry(1)
                }],
            },
            PlayerInfoPacket {
                action: UPDATE_LATENCY,
                entries: vec![PlayerInfoEntry {
                    latency: Some(75),
                    ..entry(2)
                }],
            },
            PlayerInfoPacket {
                action: UPDATE_DISPLAY_NAME,
                entries: vec![PlayerInfoEntry {
                    display_name: Some("{\"text\":\"name\"}".into()),
                    ..entry(3)
                }],
            },
            PlayerInfoPacket {
                action: REMOVE_PLAYER,
                entries: vec![entry(4), entry(5)],
            },
            PlayerInfoPacket {
                action: REMOVE_PLAYER,
                entries: Vec::new(),
            },
        ];
        for packet in cases {
            assert_eq!(decode(&encode(&packet)).unwrap(), packet);
        }
    }

    #[test]
    fn unknown_action_is_rejected() {
        let mut bytes = vec![0x05, 0x01];
        bytes.extend_from_slice(&[0; 16]);
        assert!(decode(&bytes).is_err());
    }
}
