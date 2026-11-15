//! Handshake state: `PacketHandshakingInSetProtocol` (1.8.9).
//!
//! The packet shape is the same as 26.3's `ClientIntentionPacket`; the
//! protocol field is 47 here.
use crate::{
    buf::{StrN, UShort, VarInt},
    packet, packet_enum,
};

/// The next state is status.
pub const STATUS: i32 = 1;
/// The next state is login.
pub const LOGIN: i32 = 2;

packet!(Intention {
    protocol: i32 => VarInt,
    host: String => StrN<255>,
    port: u16 => UShort,
    intent: i32 => VarInt,
});

packet_enum!(Serverbound, crate::v1_8_9::ids::handshake::serverbound {
    Intention(Intention) = INTENTION,
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intention_writes_protocol_47() {
        let p = Serverbound::Intention(Intention {
            protocol: 47,
            host: "localhost".into(),
            port: 25565,
            intent: LOGIN,
        });
        let body = p.encode();
        assert_eq!(
            body,
            [
                0x00, 0x2f, 0x09, b'l', b'o', b'c', b'a', b'l', b'h', b'o', b's', b't', 0x63, 0xdd,
                0x02,
            ]
        );
        crate::packets::round_trip(p, Serverbound::encode, Serverbound::decode);
    }
}
