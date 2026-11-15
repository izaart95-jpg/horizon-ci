//! Login state: 1.8.9's offline/online hello, compression and profile.
//!
//! Unlike 26.3 there is no configuration state and no login acknowledgement:
//! `LoginSuccess` moves the connection straight into play. The profile's UUID
//! is a 36-character string, not a 16-byte UUID.
use crate::{
    buf::{Bytes, Str, StrN, VarInt},
    packet, packet_enum,
};

packet!(
    /// `PacketLoginOutDisconnect`: a JSON chat component.
    Disconnect { reason_json: String => Str }
);
packet!(
    /// `PacketLoginOutEncryptionRequest`.
    EncryptionRequest {
        server_id: String => StrN<20>,
        public_key: Vec<u8> => Bytes,
        verify_token: Vec<u8> => Bytes,
    }
);
packet!(
    /// `PacketLoginOutSuccess`: the UUID is the dashed string form.
    LoginSuccess {
        uuid: String => StrN<36>,
        username: String => StrN<16>,
    }
);
packet!(
    /// `PacketLoginOutSetCompression`.
    SetCompression { threshold: i32 => VarInt }
);
packet!(
    /// `PacketLoginInStart`.
    LoginStart { name: String => StrN<16> }
);
packet!(
    /// `PacketLoginInEncryptionBegin`: RSA-encrypted shared secret and token.
    EncryptionResponse {
        shared_secret: Vec<u8> => Bytes,
        verify_token: Vec<u8> => Bytes,
    }
);

packet_enum!(Clientbound, crate::v1_8_9::ids::login::clientbound {
    Disconnect(Disconnect) = LOGIN_DISCONNECT,
    EncryptionRequest(EncryptionRequest) = ENCRYPTION_REQUEST,
    LoginSuccess(LoginSuccess) = LOGIN_SUCCESS,
    SetCompression(SetCompression) = SET_COMPRESSION,
});
packet_enum!(Serverbound, crate::v1_8_9::ids::login::serverbound {
    LoginStart(LoginStart) = LOGIN_START,
    EncryptionResponse(EncryptionResponse) = ENCRYPTION_RESPONSE,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::round_trip;

    #[test]
    fn login_start_bytes() {
        let p = Serverbound::LoginStart(LoginStart {
            name: "Horizon".into(),
        });
        assert_eq!(
            p.encode(),
            [0x00, 0x07, b'H', b'o', b'r', b'i', b'z', b'o', b'n']
        );
    }

    #[test]
    fn login_success_uses_a_dashed_uuid_string() {
        let uuid = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
        let p = Clientbound::LoginSuccess(LoginSuccess {
            uuid: uuid.into(),
            username: "Notch".into(),
        });
        let body = p.encode();
        assert_eq!(body[0], 0x02);
        assert_eq!(body[1], 36);
        assert_eq!(&body[2..38], uuid.as_bytes());
        assert_eq!(
            body[38..].to_vec(),
            vec![0x05, b'N', b'o', b't', b'c', b'h']
        );
        round_trip(p, Clientbound::encode, Clientbound::decode);
    }

    #[test]
    fn encryption_packets_bytes() {
        let request = Clientbound::EncryptionRequest(EncryptionRequest {
            server_id: String::new(),
            public_key: vec![1, 2, 3],
            verify_token: vec![4, 5],
        });
        assert_eq!(request.encode(), [0x01, 0x00, 0x03, 1, 2, 3, 0x02, 4, 5]);
        let response = Serverbound::EncryptionResponse(EncryptionResponse {
            shared_secret: vec![6, 7],
            verify_token: vec![8],
        });
        assert_eq!(response.encode(), [0x01, 0x02, 6, 7, 0x01, 8]);
    }

    #[test]
    fn compression_threshold_is_a_varint() {
        assert_eq!(
            Clientbound::SetCompression(SetCompression { threshold: 256 }).encode(),
            [0x03, 0x80, 0x02]
        );
    }

    #[test]
    fn login_packets_round_trip() {
        for p in [
            Clientbound::Disconnect(Disconnect {
                reason_json: "{\"text\":\"bye\"}".into(),
            }),
            Clientbound::EncryptionRequest(EncryptionRequest {
                server_id: String::new(),
                public_key: vec![1; 128],
                verify_token: vec![2; 4],
            }),
            Clientbound::LoginSuccess(LoginSuccess {
                uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".into(),
                username: "Notch".into(),
            }),
            Clientbound::SetCompression(SetCompression { threshold: 256 }),
        ] {
            round_trip(p, Clientbound::encode, Clientbound::decode);
        }
        for p in [
            Serverbound::LoginStart(LoginStart {
                name: "Horizon".into(),
            }),
            Serverbound::EncryptionResponse(EncryptionResponse {
                shared_secret: vec![9; 128],
                verify_token: vec![8; 128],
            }),
        ] {
            round_trip(p, Serverbound::encode, Serverbound::decode);
        }
    }

    #[test]
    fn rejects_trailing_bytes_and_long_names() {
        let mut body = Clientbound::LoginSuccess(LoginSuccess {
            uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".into(),
            username: "Notch".into(),
        })
        .encode();
        body.push(0);
        assert!(Clientbound::decode(0x02, &body[1..]).is_err());
        let long = Serverbound::LoginStart(LoginStart {
            name: "x".repeat(17),
        })
        .encode();
        assert!(Serverbound::decode(0x00, &long[1..]).is_err());
    }
}
