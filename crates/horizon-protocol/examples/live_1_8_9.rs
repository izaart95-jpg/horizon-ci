//! A live probe against a real 1.8.9 server (protocol 47).
//!
//! It logs in offline, answers keep-alives and the spawn teleport, then drives
//! the server as an op through commands that make it emit the clientbound
//! packets the client has to understand (chunks, entities, particles, sounds,
//! scoreboard, explosions, titles, a placed sign and its editor...). It also
//! sends the serverbound packets the game sends on join and reports every
//! packet it could not decode.
//!
//! That is how the hand-written protocol-47 codecs are checked against the
//! real server, not only against themselves.
//!
//! ```sh
//! java -jar .local/minecraft/1.8.9/server.jar nogui   # online-mode=false,
//!                                                     # network-compression-threshold=-1
//! cargo run -p horizon-protocol --example live_1_8_9 -- 127.0.0.1:25599
//! ```
//!
//! The server must list `HorizonProbe` as an op (`ops.json`) for the command
//! half of the run.
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    io::{self, Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

use horizon_protocol::{
    buf::{Reader, Writer},
    v1_8_9::{codec::Slot, handshake, login, play},
};

fn read_varint(r: &mut impl Read) -> io::Result<i32> {
    let mut value = 0i32;
    for shift in (0..35).step_by(7) {
        let mut byte = [0u8; 1];
        r.read_exact(&mut byte)?;
        value |= i32::from(byte[0] & 0x7F) << shift;
        if byte[0] & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "VarInt is longer than five bytes",
    ))
}

fn frame(body: Vec<u8>) -> Vec<u8> {
    let mut w = Writer::new();
    w.varint(body.len() as i32);
    w.bytes(&body);
    w.data
}

fn read_frame(s: &mut TcpStream) -> io::Result<(i32, Vec<u8>)> {
    let len = read_varint(s)?;
    if !(0..=2_097_151).contains(&len) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("packet length {len}"),
        ));
    }
    let mut body = vec![0u8; len as usize];
    s.read_exact(&mut body)?;
    let mut r = Reader::new(&body);
    let id = r
        .varint()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    Ok((id, r.rest().to_vec()))
}

fn send(s: &mut TcpStream, body: Vec<u8>) -> io::Result<()> {
    s.write_all(&frame(body))
}

/// The packets a full join plus this command script must produce. Sign edits
/// arrive as `UpdateSign` in 1.8.9; `TileEntityData` is only sent for a
/// non-sign block entity whose block is updated after it exists, so it is
/// not required here.
const REQUIRED: [&str; 25] = [
    "BlockChange",
    "Chat",
    "DisplayObjective",
    "EntityEffect",
    "EntityMetadata",
    "EntityStatus",
    "Explosion",
    "GameStateChange",
    "JoinGame",
    "KeepAlive",
    "MapChunk",
    "MapChunkBulk",
    "MultiBlockChange",
    "PlayerInfo",
    "PlayerPositionAndLook",
    "ScoreboardObjective",
    "ScoreboardScore",
    "SpawnMob",
    "SpawnObject",
    "SoundEffect",
    "TabCompleteResponse",
    "TimeUpdate",
    "Title",
    "WorldBorder",
    "Particles",
];

fn commands(base: [i32; 3]) -> Vec<String> {
    let [x, y, z] = base;
    vec![
        "/time set night".into(),
        "/weather thunder".into(),
        format!("/setblock {x} {y} {z} minecraft:stone"),
        format!(
            "/fill {} {} {} {} {} {} minecraft:glass",
            x - 4,
            y,
            z - 4,
            x + 4,
            y + 3,
            z + 4
        ),
        format!("/summon Zombie {x} {y} {z} {{CustomName:\"ProbeZombie\",PersistenceRequired:1b}}"),
        format!(
            "/setblock {} {} {} minecraft:chest 0 replace \
             {{Items:[{{id:\"minecraft:diamond\",Count:1,Slot:0b}}]}}",
            x + 2,
            y,
            z
        ),
        format!("/blockdata {} {y} {z} {{CustomName:\"ProbeChest\"}}", x + 2),
        format!("/setblock {} {} {} minecraft:air", x - 1, y + 4, z),
        format!(
            "/summon FallingSand {x} {} {z} {{Block:\"minecraft:sand\",Time:1}}",
            y + 4
        ),
        format!(
            "/summon Item {x} {y} {z} \
             {{Item:{{id:\"minecraft:diamond\",Count:1}},PickupDelay:32767}}"
        ),
        format!("/particle reddust {x} {y} {z} 1 1 1 0 5"),
        format!("/particle flame {x} {y} {z} 1 1 1 0.1 20"),
        format!("/playsound random.explode @p {x} {y} {z} 1 1"),
        "/effect @p speed 30 1".into(),
        "/title @p title {\"text\":\"Probe\"}".into(),
        "/scoreboard objectives add probe dummy".into(),
        "/scoreboard objectives setdisplay sidebar probe".into(),
        "/scoreboard players set @p probe 1".into(),
        format!("/worldborder center {x} {z}"),
        "/worldborder set 100".into(),
        format!("/summon LightningBolt {} {y} {}", x + 6, z + 2),
        format!(
            "/summon PrimedTnt {} {} {} {{Fuse:20}}",
            x + 4,
            y + 2,
            z + 4
        ),
        // A short move makes the server send single MapChunk packets, not
        // only the MapChunkBulk used for the initial join.
        format!("/tp @p {} {y} {}", x + 46, z + 42),
        "/kill @e[type=Zombie]".into(),
        "/tellraw @p {\"text\":\"probe done\"}".into(),
    ]
}

fn main() -> io::Result<()> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:25599".to_owned());
    let name = env::args()
        .nth(2)
        .unwrap_or_else(|| "HorizonProbe".to_owned());
    let mut s = TcpStream::connect(&addr)?;
    s.set_read_timeout(Some(Duration::from_millis(250)))?;

    send(
        &mut s,
        handshake::Serverbound::Intention(handshake::Intention {
            protocol: horizon_protocol::PROTOCOL_VERSION,
            host: "127.0.0.1".into(),
            port: 25599,
            intent: handshake::LOGIN,
        })
        .encode(),
    )?;
    send(
        &mut s,
        login::Serverbound::LoginStart(login::LoginStart { name: name.clone() }).encode(),
    )?;

    let login_deadline = Instant::now() + Duration::from_secs(15);
    let mut joined = None;
    while Instant::now() < login_deadline {
        let (id, payload) = match read_frame(&mut s) {
            Ok(p) => p,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        };
        match login::Clientbound::decode(id, &payload) {
            Ok(login::Clientbound::LoginSuccess(p)) => {
                joined = Some(p);
                break;
            }
            Ok(login::Clientbound::SetCompression(p)) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "server asked for compression at {}; start it with \
                         network-compression-threshold=-1",
                        p.threshold
                    ),
                ));
            }
            Ok(login::Clientbound::Disconnect(p)) => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("login disconnected: {}", p.reason_json),
                ));
            }
            Ok(other) => eprintln!("login: {other:?}"),
            Err(e) => eprintln!("login packet {id:#x}: {e}"),
        }
    }
    let Some(profile) = joined else {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "no login success"));
    };
    println!("joined as {} ({})", profile.username, profile.uuid);

    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut unknown: BTreeMap<i32, u32> = BTreeMap::new();
    let mut errors: Vec<(i32, String, usize)> = Vec::new();
    let mut position = [0.0f64; 3];
    let mut script: Option<Vec<String>> = None;
    let mut next_command = 0usize;
    let mut next_send = Instant::now() + Duration::from_secs(1);
    let mut tab_sent = false;
    let mut dug = 0u8;
    let mut next_dig = Instant::now() + Duration::from_secs(2);
    let mut chunks = 0u32;
    let mut biggest_bulk = 0usize;
    let mut help_sent = false;
    let mut sign_support: Option<[i32; 3]> = None;
    let mut sign_prepare_at: Option<Instant> = None;
    let mut sign_prepared = false;
    let mut sign_use_at: Option<Instant> = None;
    let mut sign_use_sent = false;
    let mut sign_edit_sent = false;

    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        // Between packets, drive the script.
        if let Some(list) = &script {
            if next_command < list.len() && Instant::now() >= next_send {
                println!("command: {}", list[next_command]);
                send(
                    &mut s,
                    play::Serverbound::Chat(play::ServerChat {
                        message: list[next_command].clone(),
                    })
                    .encode(),
                )?;
                next_command += 1;
                next_send += Duration::from_millis(350);
                // Once the glass roof is in place, give the probe a sign
                // item and place it on the roof. Placing a sign, not
                // right-clicking one, makes 1.8.9 send the editor packet.
                if next_command == 8 {
                    sign_prepare_at = Some(Instant::now() + Duration::from_millis(500));
                }
            }
            if !sign_prepared {
                if let Some(at) = sign_prepare_at {
                    if Instant::now() >= at {
                        sign_prepared = true;
                        send(
                            &mut s,
                            play::Serverbound::SetCreativeSlot(play::SetCreativeSlot {
                                slot: 36,
                                item: Slot::new(323, 1, 0),
                            })
                            .encode(),
                        )?;
                        send(
                            &mut s,
                            play::Serverbound::HeldItemChange(play::ServerHeldItemChange {
                                slot: 0,
                            })
                            .encode(),
                        )?;
                        sign_use_at = Some(Instant::now() + Duration::from_millis(200));
                    }
                }
            }
            if sign_prepared && !sign_use_sent {
                if let Some(at) = sign_use_at {
                    if Instant::now() >= at {
                        sign_use_sent = true;
                        if let Some(at) = sign_support {
                            println!("sign: place on {at:?}");
                            send(
                                &mut s,
                                play::Serverbound::BlockPlacement(play::BlockPlacement {
                                    position: at,
                                    direction: 1,
                                    held_item: Slot::new(323, 1, 0),
                                    cursor_x: 8,
                                    cursor_y: 16,
                                    cursor_z: 8,
                                })
                                .encode(),
                            )?;
                        }
                    }
                }
            }
            if next_command == list.len() && !tab_sent && Instant::now() >= next_send {
                tab_sent = true;
                send(
                    &mut s,
                    play::Serverbound::TabCompleteRequest(play::TabCompleteRequest {
                        text: "/gamemode ".into(),
                        position: Some([
                            position[0].floor() as i32,
                            position[1].floor() as i32,
                            position[2].floor() as i32,
                        ]),
                    })
                    .encode(),
                )?;
            }
        }
        if dug < 2 && Instant::now() >= next_dig {
            // A creative break: start, then finish. The server answers
            // with a block change back to air.
            let status = if dug == 0 { 0 } else { 2 };
            send(
                &mut s,
                play::Serverbound::PlayerDigging(play::PlayerDigging {
                    status,
                    position: [
                        position[0].floor() as i32,
                        position[1].floor() as i32 - 1,
                        position[2].floor() as i32,
                    ],
                    face: 1,
                })
                .encode(),
            )?;
            dug += 1;
            next_dig += Duration::from_millis(120);
        }
        let (id, payload) = match read_frame(&mut s) {
            Ok(p) => p,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                println!("server closed the connection");
                break;
            }
            Err(e) => return Err(e),
        };
        match play::Clientbound::decode(id, &payload) {
            Ok(packet) => {
                if let play::Clientbound::Unknown(id, _) = &packet {
                    *unknown.entry(*id).or_default() += 1;
                }
                let debug = format!("{packet:?}");
                let name = debug.split('(').next().unwrap_or("?").to_owned();
                *counts.entry(name.clone()).or_default() += 1;
                seen.insert(name);
                match packet {
                    play::Clientbound::KeepAlive(p) => {
                        send(&mut s, play::Serverbound::KeepAlive(p).encode())?;
                    }
                    play::Clientbound::PlayerPositionAndLook(p) => {
                        position = [p.x, p.y, p.z];
                        send(
                            &mut s,
                            play::Serverbound::PlayerPositionAndLook(
                                play::ServerPlayerPositionAndLook {
                                    x: p.x,
                                    y: p.y,
                                    z: p.z,
                                    yaw: p.yaw,
                                    pitch: p.pitch,
                                    on_ground: true,
                                },
                            )
                            .encode(),
                        )?;
                        if script.is_none() {
                            let base = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
                            sign_support = Some([base[0] - 1, base[1] + 3, base[2]]);
                            script = Some(commands(base));
                            next_send = Instant::now() + Duration::from_secs(1);
                        }
                        if !help_sent {
                            help_sent = true;
                            send(
                                &mut s,
                                play::Serverbound::Chat(play::ServerChat {
                                    message: "/help".into(),
                                })
                                .encode(),
                            )?;
                        }
                    }
                    play::Clientbound::MapChunk(p) => {
                        chunks += 1;
                        if p.ground_up && chunks == 1 {
                            println!(
                                "first single chunk {}x{}: {} bytes of section data",
                                p.chunk_x,
                                p.chunk_z,
                                p.data.len()
                            );
                        }
                    }
                    play::Clientbound::MapChunkBulk(p) => {
                        chunks += p.chunks.len() as u32;
                        biggest_bulk = biggest_bulk.max(p.data.len());
                    }
                    play::Clientbound::JoinGame(p) => {
                        println!(
                            "join: entity {} mode {} dimension {} type {:?}",
                            p.entity_id, p.gamemode, p.dimension, p.level_type
                        );
                        send(
                            &mut s,
                            play::Serverbound::ClientSettings(play::ClientSettings {
                                locale: "en_US".into(),
                                view_distance: 6,
                                chat_flags: 0,
                                chat_colors: true,
                                skin_parts: 0x7F,
                            })
                            .encode(),
                        )?;
                        send(
                            &mut s,
                            play::Serverbound::ArmAnimation(play::ArmAnimation {}).encode(),
                        )?;
                        send(
                            &mut s,
                            play::Serverbound::EntityAction(play::EntityAction {
                                entity_id: p.entity_id,
                                action: 0,
                                jump_boost: 0,
                            })
                            .encode(),
                        )?;
                        // Put a stone in the hotbar's first slot, as a
                        // creative client does, and select it.
                        send(
                            &mut s,
                            play::Serverbound::SetCreativeSlot(play::SetCreativeSlot {
                                slot: 36,
                                item: Slot {
                                    id: 1,
                                    count: 1,
                                    damage: 0,
                                    nbt: horizon_world::nbt::Tag::End,
                                },
                            })
                            .encode(),
                        )?;
                        send(
                            &mut s,
                            play::Serverbound::HeldItemChange(play::ServerHeldItemChange {
                                slot: 0,
                            })
                            .encode(),
                        )?;
                    }
                    play::Clientbound::Chat(p) => {
                        println!("chat: {}", p.message);
                    }
                    play::Clientbound::OpenSignEditor(p) => {
                        if !sign_edit_sent {
                            sign_edit_sent = true;
                            println!("sign: editor opened at {:?}", p.position);
                            send(
                                &mut s,
                                play::Serverbound::ServerUpdateSign(play::ServerUpdateSign {
                                    position: p.position,
                                    text1: "{\"text\":\"probe\"}".into(),
                                    text2: "{\"text\":\"line 2\"}".into(),
                                    text3: String::new(),
                                    text4: String::new(),
                                })
                                .encode(),
                            )?;
                        }
                    }
                    play::Clientbound::Disconnect(p) => {
                        println!("play disconnect: {}", p.reason_json);
                        break;
                    }
                    _ => {}
                }
            }
            Err(e) => {
                errors.push((id, e.to_string(), payload.len()));
                if errors.len() >= 10 {
                    break;
                }
            }
        }
    }

    println!("\nclientbound packets:");
    for (name, count) in &counts {
        println!("  {count:5}  {name}");
    }
    if !unknown.is_empty() {
        println!("unknown ids: {unknown:?}");
    }
    println!(
        "chunks {chunks}, largest bulk payload {} bytes, {} kinds of packet",
        biggest_bulk,
        counts.len()
    );
    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|name| !seen.contains(*name))
        .collect();
    if errors.is_empty() && missing.is_empty() {
        println!("all packets decoded, every required packet arrived");
        Ok(())
    } else {
        for (id, error, len) in &errors {
            println!("DECODE ERROR {id:#04x} ({len} bytes): {error}");
        }
        if !missing.is_empty() {
            println!("MISSING {missing:?}");
        }
        std::process::exit(1);
    }
}
