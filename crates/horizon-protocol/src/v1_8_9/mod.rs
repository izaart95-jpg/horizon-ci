//! Minecraft Java Edition 1.8.9 (protocol 47): the foundation of the 1.8.9
//! target described in `docs/mc-1.8.9.md`.
//!
//! 1.8.9 predates the configuration state, the data-component item format and
//! the paletted chunk format of 26.3. This module currently models the
//! handshake, status and login states plus the first play-state packets a
//! joining client needs (join game, keep alive, position, chat, disconnect).
//! The rest of the 1.8.9 play protocol, its world format and its gameplay are
//! later stages; until those land, the 26.3 modules remain the default build.
//!
//! Framing, zlib compression and AES/CFB8 encryption are shared with the 26.3
//! connection ([`crate::frame`], [`crate::connection`], [`crate::crypto`]).
pub mod codec;
pub mod entity;
pub mod handshake;
pub mod ids;
pub mod login;
pub mod play;
pub mod play_extra;
pub mod player_info;
pub mod status;

/// `SharedConstants.getProtocolVersion()` for 1.8.9.
pub const PROTOCOL_VERSION: i32 = horizon_core::version_1_8_9::PROTOCOL_VERSION;
/// The version name a 1.8.9 client reports.
pub const GAME_VERSION: &str = horizon_core::version_1_8_9::VERSION_ID;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{Connection, Pipe};
    use std::time::Duration;

    /// The full 1.8.9 login handshake over an in-memory connection, including
    /// the threshold switch and a compressed keep-alive exchange.
    #[test]
    fn login_and_join_over_a_compressed_pipe() {
        let (client_end, server_end) = Pipe::pair();
        let server = std::thread::spawn(move || {
            let mut c = Connection::local(server_end);
            c.reader
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();

            let (id, body) = c.recv().unwrap();
            match handshake::Serverbound::decode(id, &body).unwrap() {
                handshake::Serverbound::Intention(p) => {
                    assert_eq!(p.protocol, 47);
                    assert_eq!(p.host, "localhost");
                    assert_eq!(p.port, 25565);
                    assert_eq!(p.intent, handshake::LOGIN);
                }
                other => panic!("unexpected handshake: {other:?}"),
            }

            let (id, body) = c.recv().unwrap();
            assert_eq!(
                login::Serverbound::decode(id, &body).unwrap(),
                login::Serverbound::LoginStart(login::LoginStart {
                    name: "Horizon".into(),
                })
            );

            c.send(
                &login::Clientbound::SetCompression(login::SetCompression { threshold: 1 })
                    .encode(),
            )
            .unwrap();
            c.set_compression(1);
            c.send(
                &login::Clientbound::LoginSuccess(login::LoginSuccess {
                    uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".into(),
                    username: "Horizon".into(),
                })
                .encode(),
            )
            .unwrap();
            c.send(
                &play::Clientbound::JoinGame(play::JoinGame {
                    entity_id: 1,
                    gamemode: 0,
                    dimension: 0,
                    difficulty: 1,
                    max_players: 20,
                    level_type: "default".into(),
                    reduced_debug_info: false,
                })
                .encode(),
            )
            .unwrap();
            c.send(&play::Clientbound::KeepAlive(play::KeepAlive { id: 42 }).encode())
                .unwrap();

            let (id, body) = c.recv().unwrap();
            assert_eq!(
                play::Serverbound::decode(id, &body).unwrap(),
                play::Serverbound::KeepAlive(play::KeepAlive { id: 42 })
            );
        });

        let mut c = Connection::local(client_end);
        c.reader
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        c.send(
            &handshake::Serverbound::Intention(handshake::Intention {
                protocol: PROTOCOL_VERSION,
                host: "localhost".into(),
                port: 25565,
                intent: handshake::LOGIN,
            })
            .encode(),
        )
        .unwrap();
        c.send(
            &login::Serverbound::LoginStart(login::LoginStart {
                name: "Horizon".into(),
            })
            .encode(),
        )
        .unwrap();

        let (id, body) = c.recv().unwrap();
        assert_eq!(
            login::Clientbound::decode(id, &body).unwrap(),
            login::Clientbound::SetCompression(login::SetCompression { threshold: 1 })
        );
        // Vanilla switches the codec immediately after decoding this packet.
        c.set_compression(1);

        let (id, body) = c.recv().unwrap();
        assert!(matches!(
            login::Clientbound::decode(id, &body).unwrap(),
            login::Clientbound::LoginSuccess(_)
        ));
        let (id, body) = c.recv().unwrap();
        assert!(matches!(
            play::Clientbound::decode(id, &body).unwrap(),
            play::Clientbound::JoinGame(_)
        ));
        let (id, body) = c.recv().unwrap();
        assert_eq!(
            play::Clientbound::decode(id, &body).unwrap(),
            play::Clientbound::KeepAlive(play::KeepAlive { id: 42 })
        );
        c.send(&play::Serverbound::KeepAlive(play::KeepAlive { id: 42 }).encode())
            .unwrap();

        server.join().unwrap();
    }
}
