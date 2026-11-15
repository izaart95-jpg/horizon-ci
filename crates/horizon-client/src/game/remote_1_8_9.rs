//! The 1.8.9 (protocol 47) play dispatcher.
//!
//! A 1.8 server sends its own packets: numeric block states, its own chunk
//! format, one metadata index space and no synced registries. This module
//! turns those into the same `Game` state the 26.3 dispatcher builds
//! (`game/remote.rs`), reusing its world, streaming and player types rather
//! than translating packets into 26.3 ones.
use super::achievements::{ACHIEVEMENTS, by_key};
use super::remote::MovePacket;
use super::stats::stat_1_8_9;
use super::{Dim, Game};
use horizon_client::Result;
use horizon_client::first_person::{Hand, SwingAnimation};
use horizon_core::geometry::Vec3;
use horizon_core::random::RandomSource;
use horizon_protocol::packets::play as current;
use horizon_protocol::v1_8_9::play;
use horizon_protocol::v1_8_9::play as play18;
use horizon_protocol::v1_8_9::{codec as codec18, play_extra as extra18};
use horizon_world::nbt::Tag;
use horizon_world::stepped_interpolation::Path;
use horizon_world::{chunk_1_8_9, legacy_1_8_9};

/// A 1.8 angle byte as degrees.
fn degrees(angle: i8) -> f32 {
    f32::from(angle) * 360.0 / 256.0
}

/// A 1.8 fixed-point position (`1/32` block) as a coordinate.
fn fixed(value: i32) -> f64 {
    f64::from(value) / 32.0
}

/// A 1.8 velocity (`1/8000` block per tick) in blocks per tick.
fn velocity(v: [i16; 3]) -> [f64; 3] {
    v.map(|c| f64::from(c) / 8000.0)
}

/// An NBT number as an integer, whatever width the writer chose.
fn tag_number(tag: Option<&Tag>) -> Option<i64> {
    match tag? {
        Tag::Byte(v) => Some(i64::from(*v)),
        Tag::Short(v) => Some(i64::from(*v)),
        Tag::Int(v) => Some(i64::from(*v)),
        Tag::Long(v) => Some(*v),
        _ => None,
    }
}

/// A 1.8 mob id (`PacketSpawnMob`) as a current entity type name.
fn mob_1_8_9(kind: u8) -> Option<&'static str> {
    Some(match kind {
        50 => "minecraft:creeper",
        51 => "minecraft:skeleton",
        52 => "minecraft:spider",
        53 => "minecraft:giant",
        54 => "minecraft:zombie",
        55 => "minecraft:slime",
        56 => "minecraft:ghast",
        57 => "minecraft:zombified_piglin",
        58 => "minecraft:enderman",
        59 => "minecraft:cave_spider",
        60 => "minecraft:silverfish",
        61 => "minecraft:blaze",
        62 => "minecraft:magma_cube",
        63 => "minecraft:ender_dragon",
        64 => "minecraft:wither",
        65 => "minecraft:bat",
        66 => "minecraft:witch",
        67 => "minecraft:endermite",
        68 => "minecraft:guardian",
        90 => "minecraft:pig",
        91 => "minecraft:sheep",
        92 => "minecraft:cow",
        93 => "minecraft:chicken",
        94 => "minecraft:squid",
        95 => "minecraft:wolf",
        96 => "minecraft:mooshroom",
        97 => "minecraft:snow_golem",
        98 => "minecraft:ocelot",
        99 => "minecraft:iron_golem",
        100 => "minecraft:horse",
        101 => "minecraft:rabbit",
        120 => "minecraft:villager",
        _ => return None,
    })
}

/// A 1.8 `PacketSpawnObject` type as a current entity type name.
fn object_1_8_9(kind: i8) -> Option<&'static str> {
    Some(match kind {
        1 => "minecraft:boat",
        2 => "minecraft:item",
        10 => "minecraft:minecart",
        11 => "minecraft:chest_minecart",
        12 => "minecraft:furnace_minecart",
        13 => "minecraft:tnt_minecart",
        14 => "minecraft:hopper_minecart",
        15 => "minecraft:spawner_minecart",
        16 => "minecraft:command_block_minecart",
        50 => "minecraft:tnt",
        51 => "minecraft:end_crystal",
        60 => "minecraft:arrow",
        61 => "minecraft:snowball",
        62 => "minecraft:egg",
        63 => "minecraft:fireball",
        64 => "minecraft:small_fireball",
        65 => "minecraft:ender_pearl",
        66 => "minecraft:wither_skull",
        70 => "minecraft:falling_block",
        71 => "minecraft:item_frame",
        72 => "minecraft:eye_of_ender",
        73 => "minecraft:potion",
        75 => "minecraft:experience_bottle",
        76 => "minecraft:firework_rocket",
        77 => "minecraft:leash_knot",
        78 => "minecraft:armor_stand",
        90 => "minecraft:fishing_bobber",
        _ => return None,
    })
}

/// A 1.8 `EnumParticleTypes` id as the current particle it became.
fn particle_1_8_9(id: i32) -> Option<horizon_client::particles::ParticleType> {
    use horizon_client::particles::ParticleType as P;
    Some(match id {
        // `EnumParticleTypes` order: 0 explode, 1 largeexplode, 2 hugeexplosion.
        0 => P::Poof,
        1 => P::Explosion,
        2 => P::ExplosionEmitter,
        3 => P::Firework,
        4 => P::Bubble,
        5 => P::Splash,
        // 6 wake, 7 suspended, 8 depthsuspend: this engine has one underwater
        // sprite for all three.
        6 | 7 | 8 => P::Underwater,
        9 => P::Crit,
        10 => P::EnchantedHit,
        11 => P::Smoke,
        12 => P::LargeSmoke,
        13 => P::Effect,
        14 => P::InstantEffect,
        15 | 16 => P::EntityEffect,
        17 => P::Witch,
        18 => P::DrippingWater,
        19 => P::DrippingLava,
        20 => P::AngryVillager,
        21 => P::HappyVillager,
        22 => P::Mycelium,
        23 => P::Note,
        24 => P::Portal,
        25 => P::Enchant,
        26 => P::Flame,
        27 => P::Lava,
        29 => P::Cloud,
        30 => P::Dust,
        // 31 snowballpoof and 32 snowshovel both became item_snowball in 1.13.
        31 | 32 => P::ItemSnowball,
        33 => P::ItemSlime,
        34 => P::Heart,
        39 => P::FallingWater,
        // 28 footstep, 35 barrier, 36-38 item/block args, 40 take and
        // 41 mobappearance are built by `remote_particles_1_8_9` or have no
        // current particle type.
        _ => return None,
    })
}

/// 1.8's `PacketPlayInChat` reads a string of at most 100 UTF-16 units.
fn chat_1_8_9(message: &str) -> String {
    if message.encode_utf16().count() <= 100 {
        return message.to_owned();
    }
    let mut out = String::new();
    let mut units = 0;
    for ch in message.chars() {
        let n = ch.len_utf16();
        if units + n > 100 {
            break;
        }
        out.push(ch);
        units += n;
    }
    out
}

impl Game<'_> {
    pub(super) fn handle_packet_1_8_9(&mut self, packet: play::Clientbound) -> Result<()> {
        match packet {
            // Answered on the network thread before the packet got here.
            play::Clientbound::KeepAlive(_) => {}
            play::Clientbound::JoinGame(p) => self.remote_join_1_8_9(&p),
            play::Clientbound::PlayerPositionAndLook(p) => self.remote_teleport_1_8_9(&p)?,
            play::Clientbound::MapChunk(p) => self.remote_chunk_1_8_9(&p),
            play::Clientbound::MapChunkBulk(p) => self.remote_chunks_bulk_1_8_9(&p),
            play::Clientbound::BlockChange(p) => self.remote_block_1_8_9(p.position, p.block_state),
            play::Clientbound::MultiBlockChange(p) => {
                for record in &p.records {
                    self.remote_block_1_8_9(
                        [
                            p.chunk_x * 16 + i32::from(record.x),
                            i32::from(record.y),
                            p.chunk_z * 16 + i32::from(record.z),
                        ],
                        record.state,
                    );
                }
            }
            play::Clientbound::TimeUpdate(p) => {
                self.world.game_time = p.world_age;
                self.world.day_time = p.time_of_day;
            }
            // `NetHandlerPlayClient.handleSpawnPosition`: where a compass points.
            play::Clientbound::SpawnPosition(p) => {
                self.spawn = Some(Vec3::new(
                    f64::from(p.position[0]) + 0.5,
                    f64::from(p.position[1]),
                    f64::from(p.position[2]) + 0.5,
                ));
            }
            play::Clientbound::Chat(p) => self.remote_chat_1_8_9(&p),
            play::Clientbound::Disconnect(p) => {
                let reason = horizon_client::net::json_text(
                    &serde_json::from_str(&p.reason_json)
                        .unwrap_or_else(|_| serde_json::Value::String(p.reason_json.clone())),
                    &Default::default(),
                );
                self.leave_server(Some(reason))?;
            }
            play::Clientbound::UpdateHealth(p) => {
                self.remote_health_1_8_9(p.health, p.food, p.saturation)
            }
            play::Clientbound::Respawn(p) => {
                self.remote_respawn_1_8_9(p.dimension, p.difficulty, p.gamemode)
            }
            play::Clientbound::PluginMessage(p) => match p.channel.as_str() {
                "MC|Brand" => {
                    if let Ok(brand) = horizon_protocol::buf::Reader::new(&p.data).string(32767)
                        && let Some(r) = self.remote.as_mut()
                    {
                        r.brand = brand;
                    }
                }
                // The merchant window's offers; 1.8 has no packet of its own.
                "MC|TrList" => self.remote_trade_list_1_8_9(&p.data),
                // `handleCustomPayload`: a written book in the held hand.
                "MC|BOpen" => self.remote_open_book(0),
                _ => {}
            },
            play::Clientbound::ConfirmTransaction(p) => {
                self.remote_confirm_transaction_1_8_9(p.window_id, p.action)
            }
            play::Clientbound::HeldItemChange(p) => {
                self.selected = p.slot.rem_euclid(9) as usize;
            }
            play::Clientbound::GameStateChange(p) => self.remote_game_state_1_8_9(&p),
            play::Clientbound::SpawnMob(p) => self.remote_spawn_mob_1_8_9(&p),
            play::Clientbound::SpawnPlayer(p) => self.remote_spawn_player_1_8_9(&p),
            play::Clientbound::SpawnObject(p) => self.remote_spawn_object_1_8_9(&p),
            play::Clientbound::SpawnExperienceOrb(p) => {
                self.spawn_entity(
                    p.entity_id,
                    "minecraft:experience_orb",
                    [fixed(p.x), fixed(p.y), fixed(p.z)],
                    [0.0; 3],
                );
            }
            play::Clientbound::SpawnPainting(p) => {
                self.spawn_remote_object(
                    p.entity_id,
                    "minecraft:painting",
                    p.position.map(|v| f64::from(v) + 0.5),
                    degrees((p.direction as i8) * 64),
                    i32::from(p.direction),
                );
            }
            play::Clientbound::EntityRelativeMove(p) => {
                if let Some(base) = self.remote_entity_1_8_9(p.entity_id) {
                    let dx = f64::from(p.dx) / 32.0;
                    let dy = f64::from(p.dy) / 32.0;
                    let dz = f64::from(p.dz) / 32.0;
                    self.move_entity(
                        p.entity_id,
                        Some(Path::Linear([base[0] + dx, base[1] + dy, base[2] + dz])),
                        None,
                        p.on_ground,
                        MovePacket::Move,
                    );
                }
            }
            play::Clientbound::EntityLook(p) => {
                self.move_entity(
                    p.entity_id,
                    None,
                    Some((degrees(p.yaw), degrees(p.pitch))),
                    p.on_ground,
                    MovePacket::Move,
                );
            }
            play::Clientbound::EntityLookAndRelativeMove(p) => {
                if let Some(base) = self.remote_entity_1_8_9(p.entity_id) {
                    let dx = f64::from(p.dx) / 32.0;
                    let dy = f64::from(p.dy) / 32.0;
                    let dz = f64::from(p.dz) / 32.0;
                    self.move_entity(
                        p.entity_id,
                        Some(Path::Linear([base[0] + dx, base[1] + dy, base[2] + dz])),
                        Some((degrees(p.yaw), degrees(p.pitch))),
                        p.on_ground,
                        MovePacket::Move,
                    );
                }
            }
            play::Clientbound::EntityTeleport(p) => {
                self.move_entity(
                    p.entity_id,
                    Some(Path::Linear([fixed(p.x), fixed(p.y), fixed(p.z)])),
                    Some((degrees(p.yaw), degrees(p.pitch))),
                    p.on_ground,
                    MovePacket::Teleport,
                );
            }
            play::Clientbound::EntityHeadLook(p) => {
                if let Some(r) = self.remote.as_mut() {
                    r.head_lerps.insert(p.entity_id, (degrees(p.head_yaw), 3));
                }
            }
            // 1.8's `PacketPlayOutEntity` carries no action.
            play::Clientbound::Entity(_) => {}
            play::Clientbound::EntityVelocity(p) => {
                self.remote_motion(p.entity_id, velocity(p.velocity));
            }
            play::Clientbound::DestroyEntities(p) => {
                for id in p.entity_ids {
                    if let Some(r) = self.remote.as_mut() {
                        r.level_players.remove(&id);
                        r.level_player_names.remove(&id);
                        r.level_player_textures.remove(&id);
                    }
                    self.remove_entity(id);
                }
            }
            play::Clientbound::EntityAnimation(p) => {
                self.remote_animation_1_8_9(p.entity_id, p.animation)
            }
            play::Clientbound::CollectItem(p) => {
                self.remote_collect_1_8_9(p.collected_entity_id, p.collector_entity_id)
            }
            play::Clientbound::EntityStatus(p) => {
                // `NetHandlerPlayClient.handleEntityStatus`: the status byte
                // is the modern entity-event byte for most entities, so the
                // shared handler draws the hurt, death, taming and particle
                // events.
                self.remote_entity_event(p.entity_id, p.status)
            }
            play::Clientbound::AttachEntity(p) => {
                if p.leash {
                    let holder = if self
                        .remote
                        .as_ref()
                        .is_some_and(|r| r.entity_id == p.vehicle_id)
                    {
                        Some(horizon_client::mobs::leash::Holder::Player)
                    } else {
                        Some(horizon_client::mobs::leash::Holder::Mob(
                            p.vehicle_id as u32,
                        ))
                    };
                    if let Some(m) = self.mobs.iter_mut().find(|m| m.id == p.entity_id as u32) {
                        m.extra.leash = holder;
                    } else if let Some(b) =
                        self.boats.iter_mut().find(|b| b.id == p.entity_id as u32)
                    {
                        b.leash = holder;
                    }
                } else {
                    self.remote_set_passengers(p.vehicle_id, &[p.entity_id]);
                }
            }
            play::Clientbound::SetExperience(p) => {
                self.experience = horizon_client::entities::Experience {
                    level: p.level,
                    progress: p.experience_bar,
                    total: p.total_experience,
                };
            }
            play::Clientbound::SoundEffect(p) => {
                let at = [
                    f64::from(p.x) / 8.0,
                    f64::from(p.y) / 8.0,
                    f64::from(p.z) / 8.0,
                ];
                // A name with no modern counterpart still exists in the 1.8
                // registry the game runs on (a wolf howl, say), so play it as
                // sent instead of dropping the sound.
                let event = horizon_client::sound_names_1_8_9::resolve(&p.sound_name)
                    .unwrap_or(&p.sound_name);
                self.audio
                    .play(event, Some(at), p.volume, f32::from(p.pitch) / 63.0);
            }
            play::Clientbound::Particles(p) => self.remote_particles_1_8_9(&p),
            play::Clientbound::OpenWindow(p) => self.remote_open_window_1_8_9(&p),
            play::Clientbound::CloseWindow(p) => self.remote_close_window_1_8_9(p.window_id),
            play::Clientbound::SetSlot(p) => {
                self.remote_set_slot_1_8_9(p.window_id, p.slot, &p.item)
            }
            play::Clientbound::WindowItems(p) => {
                self.remote_window_items_1_8_9(p.window_id, &p.items)
            }
            play::Clientbound::EntityMetadata(p) => {
                self.remote_metadata_1_8_9(p.entity_id, &p.metadata)
            }
            play::Clientbound::EntityEquipment(p) => {
                self.remote_equipment_1_8_9(p.entity_id, p.slot, &p.item)
            }
            play::Clientbound::Abilities(p) => self.remote_abilities_1_8_9(&p),
            play::Clientbound::EntityEffect(p) => self.remote_effect_1_8_9(&p),
            play::Clientbound::RemoveEntityEffect(p) => self.remote_remove_effect_1_8_9(&p),
            play::Clientbound::PlayerInfo(p) => self.remote_player_info_1_8_9(&p),
            play::Clientbound::PlayerListHeader(p) => self.remote_tab_header_1_8_9(&p),
            play::Clientbound::Title(p) => self.remote_title_1_8_9(&p),
            play::Clientbound::BlockAction(p) => self.remote_block_action_1_8_9(&p),
            play::Clientbound::BlockBreakAnimation(p) => self.remote_break_animation_1_8_9(&p),
            play::Clientbound::Explosion(p) => self.remote_explosion_1_8_9(&p),
            play::Clientbound::WorldEvent(p) => self.remote_world_event_1_8_9(&p),
            play::Clientbound::SpawnWeather(p) => self.remote_global_entity_1_8_9(&p),
            play::Clientbound::WindowProperty(p) => self.remote_window_property_1_8_9(&p),
            play::Clientbound::TileEntityData(p) => self.remote_tile_entity_1_8_9(&p),
            play::Clientbound::UpdateSign(p) => self.remote_update_sign_1_8_9(&p),
            play::Clientbound::OpenSignEditor(p) => self.remote_open_sign_editor_1_8_9(&p),
            play::Clientbound::Camera(p) => self.remote_camera_1_8_9(&p),
            play::Clientbound::UseBed(p) => self.remote_use_bed_1_8_9(&p),
            play::Clientbound::CombatEvent(p) => self.remote_combat_event_1_8_9(&p),
            play::Clientbound::ResourcePackSend(p) => self.remote_resource_pack_1_8_9(&p),
            play::Clientbound::ScoreboardObjective(p) => self.remote_objective_1_8_9(&p),
            play::Clientbound::ScoreboardScore(p) => self.remote_score_1_8_9(&p),
            play::Clientbound::DisplayObjective(p) => self.remote_display_objective_1_8_9(&p),
            play::Clientbound::ScoreboardTeam(p) => self.remote_team_1_8_9(&p),
            play::Clientbound::WorldBorder(p) => self.remote_world_border_1_8_9(&p),
            play::Clientbound::MapUpdate(p) => self.remote_map_update_1_8_9(&p),
            play::Clientbound::EntityProperties(p) => self.remote_properties_1_8_9(&p),
            play::Clientbound::TabCompleteResponse(p) => {
                // `NetHandlerPlayClient.handleTabComplete`: the matches replace
                // the whole input they were asked for.
                let id = self.remote.as_mut().and_then(|r| r.legacy.tab_id.take());
                if let Some(id) = id {
                    let list = p.matches.into_iter().map(|text| (text, None)).collect();
                    self.chat_server_suggestions(id, (0, usize::MAX / 2), list);
                }
            }
            play::Clientbound::Difficulty(p) => {
                self.ui.difficulty = p.difficulty.min(3);
            }
            // `ClientPlayNetworkHandler.onStatsUpdate`: each named counter
            // replaces the local value; a new achievement pops up once the
            // first response has arrived.
            play::Clientbound::Statistics(p) => {
                let mut any_achievement = false;
                for (name, value) in &p.0 {
                    let Some((ty, key)) = stat_1_8_9(name) else {
                        continue;
                    };
                    if ty == "minecraft:achievement" && *value > 0 {
                        if self.ach_notify.stats_received
                            && self.stats.get("achievement", &key) == 0
                            && let Some(index) = by_key(&key)
                        {
                            self.ach_notify.display(index);
                            if key == "openInventory" {
                                // `showInventoryAchievementHint = false` once
                                // Taking Inventory is really taken.
                                self.settings.extras.show_inventory_achievement_hint = false;
                                self.settings.save(&Self::options_path());
                            }
                        }
                        any_achievement = true;
                    }
                    self.stats
                        .values
                        .entry(ty.to_owned())
                        .or_default()
                        .insert(key, *value);
                }
                // The first response, with no achievements in it, shows the
                // raw Taking Inventory hint unless it was dismissed.
                if !self.ach_notify.stats_received
                    && !any_achievement
                    && self.settings.extras.show_inventory_achievement_hint
                    && let Some(index) = by_key("openInventory")
                {
                    let title = ACHIEVEMENTS[index].title().plain();
                    let description =
                        ACHIEVEMENTS[index].description(self.settings.keys.get("key.inventory"));
                    self.ach_notify.display_raw(index, title, description);
                }
                self.ach_notify.stats_received = true;
                // `StatsScreen.onStatsReady`: the open screen rebuilds its
                // lists from the response and goes back to the General tab.
                if self.screen == super::Screen::Stats {
                    self.stats_ui.tab_1_8 = 0;
                    self.prepare_stats_1_8();
                }
                if self.screen == super::Screen::Advancements {
                    self.ach_ui.on_stats_ready();
                }
            }
            // The entity NBT update would need reading entity NBT into the
            // tracked state; no server needs it for a normal session.
            other => log::debug!("1.8.9: unhandled packet {other:?}"),
        }
        Ok(())
    }

    /// The tracked position of entity `id`, if the server spawned it.
    fn remote_entity_1_8_9(&self, id: i32) -> Option<[f64; 3]> {
        self.remote
            .as_ref()
            .and_then(|r| r.entities.get(&id))
            .map(|t| t.position)
    }

    /// `NetHandlerPlayClient.handleJoinGame`: the level, the player's mode and
    /// the dimension. 1.8 sends no registry; the client keeps its own.
    fn remote_join_1_8_9(&mut self, p: &play::JoinGame) {
        self.remote_close_local();
        if let Some(r) = self.remote.as_mut() {
            r.joined = true;
            r.logged_in = true;
            r.entity_id = p.entity_id;
            // `PacketPlayOutJoinGame` packs hardcore into bit 3 of the mode.
            r.game_mode = i32::from(p.gamemode & 0x7);
            // 1.8 has no ChunkRadiusUpdated: the radius is the one this
            // client asked for in ClientSettings, so the local render
            // distance stays.
            r.view_distance = self.settings.render_distance;
            r.abilities = None;
        }
        self.level.hardcore = p.gamemode & 0x8 != 0;
        self.enter_1_8_9_dimension(p.dimension, p.gamemode & 0x7);
        // `NetHandlerPlayClient.handleJoinGame` ends with
        // `GameSettings.sendSettingsToServer` and the MC|Brand plugin
        // message. The server only reaches play after JoinGame, so these
        // must wait for it; sending them right after LoginSuccess is a
        // protocol error.
        let info = self
            .settings
            .client_information(super::remote::client_view_distance(
                self.settings.render_distance,
            ));
        if let Some(r) = self.remote.as_mut() {
            r.sent_info = info.clone();
        }
        self.send(current::Serverbound::ClientInformation(info));
        self.send(current::Serverbound::CustomPayload(
            horizon_protocol::packets::configuration::CustomPayload {
                channel: "MC|Brand".into(),
                data: horizon_client::net::brand(),
            },
        ));
    }

    /// A 1.8 dimension switch: a fresh level shaped by the dimension. 1.8's
    /// overworld is 0..256 and the client's level range is wider, so the
    /// columns are placed in the larger range by the chunk converter.
    fn enter_1_8_9_dimension(&mut self, dimension: i8, gamemode: u8) {
        self.dim = match dimension {
            -1 => Dim::Nether,
            1 => Dim::End,
            _ => Dim::Overworld,
        };
        let shape = match self.dim {
            Dim::Overworld => horizon_client::chunk::LevelShape::OVERWORLD,
            Dim::Nether => horizon_client::chunk::LevelShape::NETHER,
            Dim::End => horizon_client::chunk::LevelShape::END,
        };
        let (time, day, weather) = (
            self.world.game_time,
            self.world.day_time,
            self.world.weather,
        );
        let mut streaming = horizon_client::streaming::Streaming::new(
            std::sync::Arc::new(super::remote::NoTerrain),
            self.pack.context.clone(),
            self.res.threads(),
        );
        streaming.remote = true;
        streaming.radius = self.streaming.radius;
        self.streaming = streaming;
        let old = std::mem::take(&mut self.world.chunks);
        let count = old.len();
        horizon_client::chunk::free_columns(old, count);
        self.world = horizon_client::world::World::new(self.res.facts().air, 0);
        self.world.shape = shape;
        self.world.game_time = time;
        self.world.day_time = day;
        self.world.weather = weather;
        self.mobs.clear();
        self.clear_remote_objects();
        self.clear_remote_projectiles();
        self.dropped.clear();
        self.particles.clear();
        self.pickups.clear();
        self.clear_lod();
        self.pending_uploads.clear();
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.clear_sections(self.world.layout());
        }
        self.streaming.sky_light = shape.sky_light;
        self.streaming.nether_shading = self.dim == Dim::Nether;
        self.player = None;
        self.riding = None;
        self.mount = Default::default();
        if let Some(r) = self.remote.as_mut() {
            r.dimension = horizon_protocol::registry::DimensionType::vanilla(self.dim.id());
            r.loaded_sent = false;
            r.position_sender = Default::default();
            r.entities.clear();
            r.lerps.clear();
            r.cart_lerps.clear();
            r.stepped.clear();
            r.head_lerps.clear();
            r.pending_items.clear();
            r.level_players.clear();
            r.level_player_names.clear();
            r.level_player_textures.clear();
            r.chunk_center = (0, 0);
        }
        self.screen = super::Screen::Playing;
        self.title = None;
        self.stop_sounds_for_new_level();
        self.audio.set_world(self.dim.id());
        self.set_mode_id(i32::from(gamemode));
        self.apply_settings();
        self.capture(false);
    }

    /// `NetHandlerPlayClient.handlePlayerPosLook`: the absolute or relative
    /// teleport of the 1.8 position packet.
    fn remote_teleport_1_8_9(&mut self, p: &play::PlayerPositionAndLook) -> Result<()> {
        const X: i8 = 0x01;
        const Y: i8 = 0x02;
        const Z: i8 = 0x04;
        const YAW: i8 = 0x08;
        const PITCH: i8 = 0x10;
        let current = self
            .player
            .as_ref()
            .map(|pl| pl.position)
            .unwrap_or(Vec3::ZERO);
        let pick = |bit: i8, base: f64, v: f64| {
            if p.flags & bit != 0 { base + v } else { v }
        };
        let position = Vec3::new(
            pick(X, current.x, p.x),
            pick(Y, current.y, p.y),
            pick(Z, current.z, p.z),
        );
        let yaw = if p.flags & YAW != 0 {
            self.yaw + p.yaw
        } else {
            p.yaw
        };
        let pitch = (if p.flags & PITCH != 0 {
            self.pitch + p.pitch
        } else {
            p.pitch
        })
        .clamp(-90.0, 90.0);
        match self.player.as_mut() {
            Some(player) => {
                player.position = position;
                player.velocity = Vec3::ZERO;
            }
            None => {
                let mut player = horizon_client::motion::PlayerMotion::new(self.motion, position)?;
                player.spectator = self.spectator();
                player.velocity = Vec3::ZERO;
                self.player = Some(player);
                self.previous_position = position;
                self.eye = 1.62;
                self.previous_eye = 1.62;
                self.apply_remote_health();
            }
        }
        self.previous_position = position;
        self.yaw = yaw;
        self.pitch = pitch;
        self.snap_player_model_pitch();
        if let Some(r) = self.remote.as_mut() {
            r.position_sender = Default::default();
            r.loaded_sent = false;
        }
        Ok(())
    }

    /// `NetHandlerPlayClient.handleChunkData`: one column, full or a section
    /// update merged into the column already loaded.
    fn remote_chunk_1_8_9(&mut self, p: &play::MapChunk) {
        // `NetHandlerPlayClient.handleChunkData`: a full-chunk packet with an
        // empty payload is the unload, not a corrupt chunk.
        if p.ground_up && p.data.is_empty() {
            self.streaming.forget(&mut self.world, p.chunk_x, p.chunk_z);
            self.world
                .block_entities
                .retain(|k, _| (k.0 >> 4, k.2 >> 4) != (p.chunk_x, p.chunk_z));
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.remove_columns([(p.chunk_x, p.chunk_z)]);
            }
            return;
        }
        let sky = self.streaming.sky_light;
        let source = match chunk_1_8_9::from_network(
            p.chunk_x,
            p.chunk_z,
            p.primary_bit_mask,
            sky,
            p.ground_up,
            &p.data,
        ) {
            Ok(chunk) => chunk,
            Err(e) => {
                log::warn!("Horizon: 1.8 chunk {} {}: {e}", p.chunk_x, p.chunk_z);
                return;
            }
        };
        self.insert_1_8_9_column(&source, p.ground_up);
    }

    /// `NetHandlerPlayClient.handleMapChunkBulk`: the 1.8 batch of full columns.
    fn remote_chunks_bulk_1_8_9(&mut self, p: &play::MapChunkBulk) {
        let metas: Vec<(i32, i32, u16)> = p
            .chunks
            .iter()
            .map(|c| (c.x, c.z, c.primary_bit_mask))
            .collect();
        // The bulk packet's own flag, not the level's, says whether the sky
        // light array is present.
        let columns = match chunk_1_8_9::from_bulk(&p.data, &metas, p.sky_light_sent) {
            Ok(columns) => columns,
            Err(e) => {
                log::warn!("Horizon: 1.8 chunk bulk: {e}");
                return;
            }
        };
        for column in &columns {
            self.insert_1_8_9_column(column, true);
        }
    }

    /// Decode a 1.8 column into the renderer's `Chunk` and insert it. A
    /// ground-up column replaces the stored one; a section update merges into
    /// the sections it carries.
    fn insert_1_8_9_column(&mut self, source: &chunk_1_8_9::Chunk1_8, ground_up: bool) {
        let layout = self.world.layout();
        let sky = self.streaming.sky_light;
        let biomes = source
            .biomes
            .as_deref()
            .map(|b| self.biomes_1_8_9(b))
            .unwrap_or_else(|| {
                let plains = world_biomes!(self).id("minecraft:plains");
                [plains; 256]
            });
        let chunk = {
            let registry = &self.pack.assets.registry;
            horizon_client::chunk_1_8_9::chunk(source, layout, sky, &biomes, |state| {
                registry.id_filled(state).map(|id| id.0)
            })
        };
        let mut chunk = chunk;
        if !ground_up {
            // A section update carries only the sections in its mask; the
            // others keep the column's current data.
            let offset = (-layout.min_y() / 16) as usize;
            if let Some(existing) = self.world.chunks.get(&(source.x, source.z)) {
                for (i, section) in chunk.sections.iter_mut().enumerate() {
                    let sent = source
                        .sections
                        .iter()
                        .any(|s| usize::from(s.y()) + offset == i);
                    if !sent && let Some(old) = existing.sections.get(i) {
                        *section = old.clone();
                    }
                }
                chunk.block_entities = existing.block_entities.clone();
                chunk.block_names = existing.block_names.clone();
            }
        }
        self.streaming.insert_remote(&mut self.world, chunk);
    }

    /// The 1.8 per-column biome ids as the client's biome table indices.
    fn biomes_1_8_9(&self, legacy: &[u8]) -> [u16; 256] {
        let biomes = world_biomes!(self);
        let plains = biomes.id("minecraft:plains");
        let mut out = [plains; 256];
        for (i, &id) in legacy.iter().take(256).enumerate() {
            if let Some(name) = horizon_world::biome_1_8_9::biome_name(id) {
                out[i] = biomes.id(name);
            }
        }
        out
    }

    /// `NetHandlerPlayClient.handleBlockChange`: a 1.8 numeric block state.
    fn remote_block_1_8_9(&mut self, pos: [i32; 3], state: i32) {
        let id = (state >> 4) as u16;
        let meta = (state & 0x0F) as u8;
        let block = legacy_1_8_9::block_state(id, meta);
        let Some(state) = self.pack.assets.registry.id_filled(&block) else {
            log::debug!("1.8.9: no registry state for {} at {pos:?}", block.name);
            return;
        };
        let previous = self.world.block(pos[0], pos[1], pos[2]);
        if let Err(e) = self.level_set(pos, state) {
            log::warn!("1.8.9: block change at {pos:?}: {e}");
        }
        // `LevelChunk.setBlockState` client side: the previous block entity
        // goes and the new block gets its fresh one, before the tile entity
        // packet that follows the block change arrives to fill it.
        self.sync_block_entity((pos[0], pos[1], pos[2]), Some(previous), state);
    }

    /// `NetHandlerPlayClient.handleChat`: a JSON component, not NBT.
    fn remote_chat_1_8_9(&mut self, p: &play::Chat) {
        let text = match serde_json::from_str::<serde_json::Value>(&p.message) {
            Ok(value) => horizon_client::scoreboard::comp_text(&value),
            Err(_) => horizon_client::text::Text::literal(&p.message),
        };
        // 1.8 position: 0 chat, 1 system, 2 above the hotbar.
        match p.position {
            2 => {
                self.say_system_queued(&text);
                self.titles.set_overlay(text, false);
            }
            _ => {
                if self.chat_line(text.clone(), horizon_client::chat::ChatSource::Server) {
                    self.say_system_chat_queued(&text);
                }
            }
        }
    }

    fn remote_spawn_mob_1_8_9(&mut self, p: &play::SpawnMob) {
        let Some(kind) = mob_1_8_9(p.kind) else {
            log::debug!("1.8.9: unknown mob type {}", p.kind);
            return;
        };
        let position = [fixed(p.x), fixed(p.y), fixed(p.z)];
        let rotation = [degrees(p.yaw), degrees(p.pitch), degrees(p.head_pitch)];
        self.spawn_entity(p.entity_id, kind, position, rotation);
        self.remote_motion(p.entity_id, velocity(p.velocity));
        // PacketPlayOutSpawnMob carries the mob's initial entity data
        // (sheep colour, slime size, villager profession, flags, name).
        self.remote_metadata_1_8_9(p.entity_id, &p.metadata);
    }

    fn remote_spawn_player_1_8_9(&mut self, p: &play::SpawnPlayer) {
        let position = [fixed(p.x), fixed(p.y), fixed(p.z)];
        let rotation = [degrees(p.yaw), degrees(p.pitch), degrees(p.yaw)];
        if let Some(r) = self.remote.as_mut() {
            r.level_players.insert(p.entity_id, p.uuid);
            match r.players.get(&p.uuid) {
                Some(name) => r.level_player_names.insert(p.entity_id, name.clone()),
                None => r.level_player_names.remove(&p.entity_id),
            };
        }
        self.persist.uuids.insert(
            p.entity_id as u32,
            horizon_client::lan::playerdata::uuid_ints(p.uuid),
        );
        self.spawn_entity(p.entity_id, "minecraft:player", position, rotation);
        // A spawned player's skin flags and pose arrive with the spawn packet.
        self.remote_metadata_1_8_9(p.entity_id, &p.metadata);
    }

    fn remote_spawn_object_1_8_9(&mut self, p: &horizon_protocol::v1_8_9::play_extra::SpawnObject) {
        let Some(kind) = object_1_8_9(p.kind) else {
            log::debug!("1.8.9: unknown object type {}", p.kind);
            return;
        };
        let position = [
            fixed(p.position[0]),
            fixed(p.position[1]),
            fixed(p.position[2]),
        ];
        let yaw = degrees(p.yaw);
        let pitch = degrees(p.pitch);
        let velocity = p.velocity.map(velocity).unwrap_or([0.0; 3]);
        // `handleSpawnObject` type 73: the object data is the potion's damage
        // value (bit 14 makes it splash), so there is no entity-data stack to
        // wait for.
        if p.kind == 73 {
            self.spawn_entity(p.entity_id, kind, position, [yaw, pitch, yaw]);
            self.remote_motion(p.entity_id, velocity);
            self.potions.add_remote_thrown_1_8_9(
                p.entity_id as u32,
                p.object_data,
                Vec3::new(position[0], position[1], position[2]),
                Vec3::new(velocity[0], velocity[1], velocity[2]),
            );
            return;
        }
        // `Block.getStateById`: a falling block's data packs the numeric block
        // id in the low 12 bits and the metadata above it, while the shared
        // path expects a current block state id.
        let data = if kind == "minecraft:falling_block" {
            let raw = p.object_data.max(0) as u32;
            let state = horizon_world::legacy_1_8_9::block_state(
                (raw & 0x0FFF) as u16,
                ((raw >> 12) & 0x0F) as u8,
            );
            self.pack
                .assets
                .registry
                .id_filled(&state)
                .map_or(p.object_data, |id| id.0 as i32)
        } else {
            p.object_data
        };
        self.spawn_entity(p.entity_id, kind, position, [yaw, pitch, yaw]);
        self.remote_motion(p.entity_id, velocity);
        if !self.spawn_remote_object(p.entity_id, kind, position, yaw, data) {
            self.spawn_remote_projectile(p.entity_id, kind, position, (yaw, pitch), velocity, data);
        }
    }

    /// `NetHandlerPlayClient.handleAnimation`: 1.8's action numbering, which
    /// differs from the current packet's.
    fn remote_animation_1_8_9(&mut self, id: i32, action: u8) {
        match action {
            // Swing the arm.
            0 => {
                if self.remote.as_ref().is_some_and(|r| r.entity_id == id) {
                    self.swing_hand(Hand::Main, SwingAnimation::DEFAULT);
                } else if let Some(m) = self.mobs.iter_mut().find(|m| m.id == id as u32) {
                    m.swing_hand_modified(false, SwingAnimation::DEFAULT);
                }
            }
            // Leave bed.
            2 => {
                if self.remote.as_ref().is_some_and(|r| r.entity_id == id) {
                    self.sleeping = None;
                    self.sleep_timer = 100;
                }
            }
            // A critical or a magic-critical hit's particles.
            3 => self.remote_animate(id, 1),
            4 => self.remote_animate(id, 2),
            _ => {}
        }
    }

    /// `NetHandlerPlayClient.handleCollect`: the pickup sound, the particle
    /// toward the collector and the item's removal.
    fn remote_collect_1_8_9(&mut self, item: i32, collector: i32) {
        let orb = self.orbs.iter().any(|o| o.id == item as u32);
        let dropped = self
            .dropped
            .iter()
            .find(|d| d.id == item as u32)
            .map(|d| [d.position.x, d.position.y, d.position.z]);
        let shot = self
            .creatures
            .projectiles
            .iter()
            .find(|s| s.id == item as u32)
            .map(|s| [s.position.x, s.position.y, s.position.z]);
        let at = dropped.or(shot).or_else(|| {
            self.remote
                .as_ref()
                .is_some_and(|r| r.entity_id == collector)
                .then(|| self.eye_position())
                .flatten()
        });
        if !orb && let Some(at) = at {
            let pitch = ((self.world.random.next_float() - self.world.random.next_float()) * 0.7
                + 1.0)
                * 2.0;
            self.audio
                .play("minecraft:entity.item.pickup", Some(at), 0.2, pitch);
        }
        let collector = if self
            .remote
            .as_ref()
            .is_some_and(|r| r.entity_id == collector)
        {
            super::pickups::Collector::Player
        } else {
            super::pickups::Collector::Mob(collector as u32)
        };
        if let Some(item) = self.dropped.iter().find(|d| d.id == item as u32).cloned() {
            self.add_pickup(super::pickups::Taken::Item(Box::new(item)), collector);
        } else if let Some(orb) = self.orbs.iter().find(|o| o.id == item as u32).cloned() {
            self.add_pickup(super::pickups::Taken::Orb(Box::new(orb)), collector);
        } else if let Some(shot) = self
            .creatures
            .projectiles
            .iter()
            .find(|s| s.id == item as u32)
            .cloned()
        {
            self.add_pickup(super::pickups::Taken::Arrow(Box::new(shot)), collector);
        }
        // An orb stays: the server may still hold merged ones in it.
        if !orb {
            self.remove_entity(item);
        }
    }

    /// `NetHandlerPlayClient.handleParticles`: 1.8's numeric particle id.
    fn remote_particles_1_8_9(&mut self, p: &horizon_protocol::v1_8_9::play_extra::Particles) {
        use horizon_client::particles::{ParticleOptions, ParticleType};
        let at = p.position.map(f64::from);
        // Ids 35-38 carry an argument and do not use the simple sprite.
        let options = match p.particle_id {
            // `EnumParticleTypes.BARRIER`: the barrier block's own sprite,
            // untinted.
            35 => {
                let Some(state) = self.pack.assets.registry.default_state("minecraft:barrier")
                else {
                    return;
                };
                let Some(sprite) = Self::block_particle_sprite(&self.pack, state) else {
                    return;
                };
                ParticleOptions::block(ParticleType::Block, sprite, 0xFFFFFF)
            }
            // `EntityItemFX`: data[0] is the item id and data[1] its damage.
            36 => {
                let Some(stack) = p.data.first().and_then(|id| {
                    horizon_items::legacy_1_8_9::stack(
                        *id as i16,
                        1,
                        p.data.get(1).copied().unwrap_or(0) as i16,
                        &Tag::End,
                    )
                }) else {
                    return;
                };
                self.item_particle(&stack)
            }
            // `EntityBlockCrackFX` / `EntityBlockDustFX`: data[0] is
            // `Block.getStateId`, `id | meta << 12`.
            37 | 38 => {
                let Some(packed) = p.data.first().copied() else {
                    return;
                };
                let legacy = legacy_1_8_9::block_state_from_packed(packed);
                let Some(state) = self.pack.assets.registry.id_filled(&legacy) else {
                    return;
                };
                let block = at.map(|c| c.floor() as i32);
                let Some((sprite, tint)) = self.particle_sprite(state, block) else {
                    return;
                };
                ParticleOptions::block(ParticleType::Block, sprite, tint)
            }
            id => {
                let Some(ty) = particle_1_8_9(id) else {
                    return;
                };
                match id {
                    // `EntityReddustFX`: the offsets are the RGB colour, not a
                    // spread, and the particle has no velocity of its own.
                    30 => ParticleOptions::dust(p.offset, 1.0),
                    // `EntitySpellParticleFX` for mobSpell/mobSpellAmbient:
                    // data is the colour in 0-255 channels.
                    15 | 16 if p.data.len() >= 3 => {
                        let channel = |i: usize| p.data[i].clamp(0, 255) as f32 / 255.0;
                        ParticleOptions {
                            color: [channel(0), channel(1), channel(2), 1.0],
                            ..ParticleOptions::simple(ty)
                        }
                    }
                    _ => ParticleOptions::simple(ty),
                }
            }
        };
        // The colour-carrying ids put the colour in the offsets; they must not
        // also spread or drift the particle.
        let coloured = matches!(p.particle_id, 15 | 16 | 30);
        if p.count <= 0 {
            let v = if coloured {
                [0.0; 3]
            } else {
                p.offset.map(|o| f64::from(o) * f64::from(p.speed))
            };
            self.particles
                .add_with(&options, at, v, p.long_distance, false);
            return;
        }
        for _ in 0..p.count.min(256) {
            let mut jitter = |o: f32| f64::from((self.world.random.next_float() - 0.5) * o);
            let position = [
                at[0] + jitter(p.offset[0]),
                at[1] + jitter(p.offset[1]),
                at[2] + jitter(p.offset[2]),
            ];
            let mut v =
                |o: f32| f64::from((self.world.random.next_float() - 0.5) * o + o * p.speed);
            let velocity = [v(p.offset[0]), v(p.offset[1]), v(p.offset[2])];
            self.particles
                .add_with(&options, position, velocity, p.long_distance, false);
        }
    }

    fn remote_health_1_8_9(&mut self, health: f32, food: i32, saturation: f32) {
        let flash = self.remote.as_mut().is_some_and(|r| {
            r.health = (health, food, saturation);
            std::mem::replace(&mut r.flash_on_set_health, true)
        });
        if flash && let Some(pl) = self.player.as_mut() {
            pl.health.flash_health_change(health);
        }
        self.apply_remote_health();
        let shown = self.remote.as_ref().is_none_or(|r| r.show_death_screen);
        if health <= 0.0 && shown && self.screen != super::Screen::Dead && self.player.is_some() {
            self.close_dialogs();
            self.screen = super::Screen::Dead;
            self.ui.death_time = 0;
            self.ui.death_confirm = None;
            self.capture(false);
        }
    }

    /// `NetHandlerPlayClient.handleRespawn`: the server moves us to a level.
    ///
    /// 1.8 sends this on a dimension change and on a death respawn. Only a
    /// dimension change builds a new world (`WorldClient`); a same-dimension
    /// respawn removes the old player entity and keeps the loaded chunks.
    fn remote_respawn_1_8_9(&mut self, dimension: i32, difficulty: u8, gamemode: u8) {
        // 1.8's respawn repeats the world's difficulty and packs hardcore into
        // the mode, as JoinGame does.
        self.ui.difficulty = difficulty.min(3);
        let gamemode = gamemode & 0x7;
        let dim = match dimension {
            -1 => Dim::Nether,
            1 => Dim::End,
            _ => Dim::Overworld,
        };
        // A new `EntityPlayerSP`: no effects, no item in use, no vehicle.
        self.potions.effects.clear();
        self.using = None;
        self.riding = None;
        self.mount = Default::default();
        if let Some(r) = self.remote.as_mut() {
            r.own_invisible = false;
        }
        if dim == self.dim {
            self.player = None;
            self.set_mode_id(i32::from(gamemode));
        } else {
            self.enter_1_8_9_dimension(dimension as i8, gamemode);
            // `LevelExtractor.setLevel`: a new level drops the highlights.
            self.test_highlights.clear();
            let (music, _) = self.situational_music();
            self.music.stop_playing(music.as_ref(), &mut self.audio);
        }
        self.screen = super::Screen::Playing;
        self.ui.death_confirm = None;
        if let Some(r) = self.remote.as_mut() {
            r.death_message = None;
            r.health = (20.0, 20, 5.0);
            // handleRespawn: the new LocalPlayer is the camera.
            r.camera = None;
        }
        self.capture(true);
    }

    /// `NetHandlerPlayClient.handleEntityProperties`: 1.8 names attributes
    /// (`generic.maxHealth`) where 26.3 uses registry names
    /// (`minecraft:max_health`), and names modifiers by UUID. Only the local
    /// player's values drive movement and the hearts the HUD draws.
    fn remote_properties_1_8_9(&mut self, p: &extra18::EntityProperties) {
        if !self
            .remote
            .as_ref()
            .is_some_and(|r| r.entity_id == p.entity_id)
        {
            return;
        }
        let catalog = &self.motion.attributes;
        let values: Vec<current::AttributeSnapshot> = p
            .properties
            .iter()
            .filter_map(|property| {
                let attribute = catalog.id(attribute_1_8_9(&property.key)?)?;
                Some(current::AttributeSnapshot {
                    attribute: attribute.registry_id(),
                    base: property.value,
                    modifiers: property
                        .modifiers
                        .iter()
                        .map(|m| current::AttributeModifier {
                            // `Modifier.new` wants a resource location; the UUID
                            // is the name a 1.8 server knows the modifier by.
                            id: format!("legacy:{:032x}", m.uuid),
                            amount: m.amount,
                            operation: i32::from(m.operation),
                        })
                        .collect(),
                })
            })
            .collect();
        if let Some(player) = self.player.as_mut() {
            horizon_client::lan::attributes::apply(&mut player.attributes, &values);
        }
        self.apply_remote_health();
    }

    fn remote_game_state_1_8_9(&mut self, p: &play::GameStateChange) {
        match p.reason {
            // 1: rain begins, 2: rain ends.
            1 => self.world.weather.raining = true,
            2 => self.world.weather.raining = false,
            // 3: the player's game mode changed.
            3 => self.set_mode_id(p.value as i32),
            _ => {}
        }
    }
}

/// 1.8's five `EntityVillager` professions as indices into the modern
/// `render::PROFESSIONS` table (farmer, librarian, cleric, armorer, butcher).
fn legacy_profession(profession: i32) -> u8 {
    match profession {
        0 => 5,
        1 => 9,
        2 => 4,
        3 => 1,
        4 => 2,
        _ => 0,
    }
}

/// A 1.8 `IAttribute` name (`AttributeMap`'s keys are translatable names) as
/// the 26.3 registry name for the same attribute.
fn attribute_1_8_9(key: &str) -> Option<&'static str> {
    Some(match key {
        "generic.maxHealth" => "minecraft:max_health",
        "generic.followRange" => "minecraft:follow_range",
        "generic.knockbackResistance" => "minecraft:knockback_resistance",
        "generic.movementSpeed" => "minecraft:movement_speed",
        "generic.attackDamage" => "minecraft:attack_damage",
        "generic.attackSpeed" => "minecraft:attack_speed",
        "generic.armor" => "minecraft:armor",
        "generic.armorToughness" => "minecraft:armor_toughness",
        "generic.flyingSpeed" => "minecraft:flying_speed",
        "horse.jumpStrength" => "minecraft:jump_strength",
        _ => return None,
    })
}

/// A 1.8 slot as a current stack.
fn item_1_8_9(slot: &codec18::Slot) -> Option<horizon_client::items::ItemStack> {
    horizon_items::legacy_1_8_9::stack(slot.id, slot.count, slot.damage, &slot.nbt)
}

/// A current stack as a 1.8 slot, empty when 1.8 had no such item.
fn slot_1_8_9(stack: &Option<horizon_client::items::ItemStack>) -> codec18::Slot {
    let Some(stack) = stack else {
        return codec18::Slot::empty();
    };
    match horizon_items::legacy_1_8_9::legacy_stack(stack) {
        Some((id, count, damage, nbt)) => {
            let mut slot = codec18::Slot::new(id, count, damage);
            slot.nbt = nbt;
            slot
        }
        None => codec18::Slot::empty(),
    }
}

impl Game<'_> {
    /// The 1.8.9 form of one of the game's serverbound packets.
    ///
    /// The gameplay code builds its network actions as 26.3 packets; a 1.8.9
    /// build has to put them on the wire as protocol 47 ones. This is the one
    /// place that translates between the two, and it is temporary: porting the
    /// gameplay send sites to the 1.8 vocabulary removes it (PORTING.md).
    pub(super) fn serverbound_1_8_9(
        &self,
        packet: &current::Serverbound,
    ) -> Option<play18::Serverbound> {
        use current::Serverbound as C;
        use extra18::UseEntity;
        use play18::Serverbound as P;
        Some(match packet {
            C::Chat(p) => P::Chat(play18::ServerChat {
                message: chat_1_8_9(&p.message),
            }),
            // 1.8 has no command packet: a command is a slash-prefixed chat.
            C::ChatCommand(p) => P::Chat(play18::ServerChat {
                message: chat_1_8_9(&format!("/{}", p.command)),
            }),
            C::ChatCommandSigned(p) => P::Chat(play18::ServerChat {
                message: chat_1_8_9(&format!("/{}", p.command)),
            }),
            C::ClientCommand(p) => match p.action {
                0 | 1 => P::ClientCommand(play18::ClientCommand { action: p.action }),
                _ => return None,
            },
            C::Punch(_) => P::ArmAnimation(play18::ArmAnimation {}),
            C::Attack(p) => P::UseEntity(UseEntity::Attack { target: p.id }),
            // 26.3 always carries the hit point, relative to the entity, as
            // 1.8's INTERACT_AT does; Entity.interactAt forwards to interact
            // for the mobs that do not read it, so no plain interaction is
            // lost.
            C::Interact(p) => P::UseEntity(UseEntity::InteractAt {
                target: p.id,
                x: p.location[0] as f32,
                y: p.location[1] as f32,
                z: p.location[2] as f32,
            }),
            C::UseItemOn(p) => P::BlockPlacement(play18::BlockPlacement {
                position: p.pos,
                direction: p.face as i8,
                held_item: slot_1_8_9(&self.held().cloned()),
                // 26.3's BlockHitResult carries the point inside the block as
                // 0..1; 1.8's PacketPlayInBlockPlace carries 0..16.
                cursor_x: (p.cursor[0].clamp(0.0, 1.0) * 16.0) as i8,
                cursor_y: (p.cursor[1].clamp(0.0, 1.0) * 16.0) as i8,
                cursor_z: (p.cursor[2].clamp(0.0, 1.0) * 16.0) as i8,
            }),
            // `PlayerControllerMP.sendUseItem`: a block placement at the
            // origin block -1,-1,-1 with direction 255 (as a byte, -1).
            C::UseItem(_) => P::BlockPlacement(play18::BlockPlacement {
                position: [-1, -1, -1],
                direction: -1,
                held_item: slot_1_8_9(&self.held().cloned()),
                cursor_x: 0,
                cursor_y: 0,
                cursor_z: 0,
            }),
            // ServerboundPlayerActionPacket.Action to PacketPlayInBlockDig:
            // 1.8 swaps the two drop actions and has no offhand.
            C::PlayerAction(p) => {
                // 26.3: 0 start, 2 abort, 3 stop, 4 drop all, 5 drop one,
                // 6 release use. 1.8: 0 start, 1 abort, 2 stop, 3 drop all,
                // 4 drop one, 5 release use.
                let status = match p.action {
                    0 => 0,
                    2 => 1,
                    3 => 2,
                    4 => 3,
                    5 => 4,
                    6 => 5,
                    _ => return None,
                };
                P::PlayerDigging(play18::PlayerDigging {
                    status,
                    position: p.pos,
                    face: p.direction as i8,
                })
            }
            C::SetCarriedItem(p) => {
                P::HeldItemChange(play18::ServerHeldItemChange { slot: p.slot })
            }
            C::SetCreativeModeSlot(p) => P::SetCreativeSlot(play18::SetCreativeSlot {
                slot: p.slot,
                item: slot_1_8_9(&self.to_stack(&p.item)),
            }),
            C::ContainerClose(p) => P::ServerCloseWindow(play18::ServerCloseWindow {
                window_id: p.container.clamp(0, 255) as u8,
            }),
            C::ContainerButtonClick(p) => P::EnchantItem(play18::EnchantItem {
                window_id: p.container as i8,
                enchantment: p.button as i8,
            }),
            C::SignUpdate(p) => {
                let line = |text: &String| serde_json::json!({ "text": text }).to_string();
                P::ServerUpdateSign(play18::ServerUpdateSign {
                    position: p.pos,
                    text1: line(&p.lines[0]),
                    text2: line(&p.lines[1]),
                    text3: line(&p.lines[2]),
                    text4: line(&p.lines[3]),
                })
            }
            C::RenameItem(p) => {
                // AnvilGUI: `PacketBuffer.writeString(name)`, so the UTF-8
                // bytes carry a VarInt length inside the payload.
                let mut w = horizon_protocol::buf::Writer::new();
                w.string(&p.name);
                P::PluginMessage(play18::PluginMessage {
                    channel: "MC|ItemName".into(),
                    data: w.data,
                })
            }
            C::SelectTrade(p) => P::PluginMessage(play18::PluginMessage {
                channel: "MC|TrSel".into(),
                data: p.item.to_be_bytes().to_vec(),
            }),
            C::SetBeacon(p) => {
                let power = |p: Option<i32>| p.unwrap_or(0);
                let mut data = power(p.primary).to_be_bytes().to_vec();
                data.extend_from_slice(&power(p.secondary).to_be_bytes());
                P::PluginMessage(play18::PluginMessage {
                    channel: "MC|Beacon".into(),
                    data,
                })
            }
            C::CustomPayload(p) => P::PluginMessage(play18::PluginMessage {
                channel: p.channel.clone(),
                data: p.data.clone(),
            }),
            // CommandBlockLogic.func_145757_a: MC|AdvCdm identifies the block
            // (byte 0, position) or the minecart (byte 1, entity id), then the
            // command. 1.8 has no mode, conditional or auto fields.
            C::SetCommandBlock(p) => {
                let mut w = horizon_protocol::buf::Writer::new();
                w.i8(0);
                for v in p.pos {
                    w.i32(v);
                }
                w.string(&p.command);
                P::PluginMessage(play18::PluginMessage {
                    channel: "MC|AdvCdm".into(),
                    data: w.data,
                })
            }
            C::SetCommandMinecart(p) => {
                let mut w = horizon_protocol::buf::Writer::new();
                w.i8(1);
                w.i32(p.entity);
                w.string(&p.command);
                P::PluginMessage(play18::PluginMessage {
                    channel: "MC|AdvCdm".into(),
                    data: w.data,
                })
            }
            // BookEditScreen.saveChanges: 1.8 sends the whole book as an item
            // stack on MC|BEdit, or MC|BSign with the title and author.
            C::EditBook(p) => {
                let stack = self.stack_in(p.slot.max(0) as usize).cloned();
                let mut item = slot_1_8_9(&stack);
                if item.is_empty() {
                    return None;
                }
                let pages = |pages: &[String]| {
                    Tag::List(
                        pages
                            .iter()
                            .map(|page| Tag::String(page.as_str().into()))
                            .collect(),
                    )
                };
                if !matches!(item.nbt, Tag::Compound(_)) {
                    item.nbt = Tag::Compound(Default::default());
                }
                if let Tag::Compound(entries) = &mut item.nbt {
                    entries.insert("pages".into(), pages(&p.pages));
                    if let Some(title) = &p.title {
                        entries.insert("title".into(), Tag::String(title.as_str().into()));
                        entries.insert(
                            "author".into(),
                            Tag::String(super::chat::PLAYER_NAME.into()),
                        );
                    }
                }
                let mut w = horizon_protocol::buf::Writer::new();
                {
                    use horizon_protocol::buf::Codec as _;
                    codec18::Slot::write(&item, &mut w);
                }
                P::PluginMessage(play18::PluginMessage {
                    channel: if p.title.is_some() {
                        "MC|BSign".into()
                    } else {
                        "MC|BEdit".into()
                    },
                    data: w.data,
                })
            }
            // EntityPlayerSP.sendPlayerAbilities: 1.8's PacketPlayInAbilities
            // also carries the two speeds; the defaults are the field values.
            C::PlayerAbilities(p) => {
                // PacketPlayInAbilities echoes `PlayerCapabilities`, whose
                // speeds the server may have changed; fall back to the
                // defaults only before the first Abilities packet.
                let abilities = self.remote.as_ref().and_then(|r| r.abilities.as_ref());
                P::ServerAbilities(play18::ServerAbilities {
                    flags: p.flags,
                    flying_speed: abilities.map_or(0.05, |a| a.flying_speed),
                    walking_speed: abilities.map_or(0.1, |a| a.walking_speed),
                })
            }
            // A spectator clicking a player (ServerboundTeleportToEntityPacket)
            // is 1.8's Spectate packet, by UUID.
            C::TeleportToEntity(p) => P::Spectate(play18::Spectate { target: p.uuid }),
            C::ClientInformation(p) => P::ClientSettings(play18::ClientSettings {
                locale: p.language.clone(),
                view_distance: p.view_distance,
                chat_flags: p.chat_visibility as i8,
                chat_colors: p.chat_colors,
                skin_parts: p.model_customisation,
            }),
            C::PlayerCommand(p) => {
                // 26.3: 0 stop sleeping, 1/2 sprint, 3/4 riding jump, 5 open
                // inventory, 6 start fall flying. 1.8: 2 stop sleeping, 3/4
                // sprint, 5/6 riding jump; it has no gliding or inventory action.
                let action = match p.action {
                    0 => 2,
                    1 => 3,
                    2 => 4,
                    3 => 5,
                    4 => 6,
                    _ => return None,
                };
                P::EntityAction(play18::EntityAction {
                    entity_id: p.id,
                    action,
                    // 26.3 leaves the jump scale in the packet's data word;
                    // 1.8's START_RIDING_JUMP carries it as jumpBoost.
                    jump_boost: if p.action == 3 { p.data } else { 0 },
                })
            }
            // `ClientSuggestionProvider.customSuggestion` on a 1.8 server is
            // `C14PacketTabComplete`, whose body is the id and the whole input.
            C::Unknown(id, data)
                if *id == horizon_protocol::ids::play::serverbound::COMMAND_SUGGESTION =>
            {
                let mut r = horizon_protocol::buf::Reader::new(data);
                let _id = r.varint().ok()?;
                let text = r.string(32767).ok()?;
                P::TabCompleteRequest(play18::TabCompleteRequest {
                    text,
                    position: None,
                })
            }
            // The 26.3-only actions: the server predicts from these packets,
            // while 1.8's server keeps its own state.
            _ => return None,
        })
    }

    /// `EntityPlayerSP.onUpdateWalkingPlayer` for 1.8.9: the sprint and sneak
    /// actions when they change, the held slot, then the one movement packet
    /// that carries what moved — a position past 1/32 of a block or every 20
    /// ticks, and the look whenever it turned.
    pub(super) fn remote_send_state_1_8_9(&mut self, input: u8) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let position = [p.position.x, p.position.y, p.position.z];
        let on_ground = p.ground.on_ground;
        let sprinting = p.sprinting;
        let (yaw, pitch, selected) = (self.yaw, self.pitch, self.selected);
        let riding = self.riding.is_some();
        let controls_camera = self.controls_camera();
        let Some(r) = self.remote.as_mut() else {
            return;
        };
        let sneaking = input & 0x20 != 0;
        let mut out = Vec::new();
        if r.legacy.carried != Some(selected) {
            r.legacy.carried = Some(selected);
            out.push(play18::Serverbound::HeldItemChange(
                play18::ServerHeldItemChange {
                    slot: selected as i16,
                },
            ));
        }
        if sprinting != r.legacy.sprinting {
            r.legacy.sprinting = sprinting;
            out.push(play18::Serverbound::EntityAction(play18::EntityAction {
                entity_id: r.entity_id,
                action: if sprinting { 3 } else { 4 },
                jump_boost: 0,
            }));
        }
        if sneaking != r.legacy.sneaking {
            r.legacy.sneaking = sneaking;
            out.push(play18::Serverbound::EntityAction(play18::EntityAction {
                entity_id: r.entity_id,
                action: if sneaking { 0 } else { 1 },
                jump_boost: 0,
            }));
        }
        if riding {
            // EntityPlayerSP.onUpdate: a passenger sends its look and the
            // vehicle's input every tick.
            let bit = |n: u8| f32::from(input & (1 << n) != 0);
            let (dx, dz) = (bit(2) - bit(3), bit(0) - bit(1));
            let len = (dx * dx + dz * dz).sqrt();
            // MovementInputFromOptions normalises a diagonal, then scales the
            // keys to 0.98.
            let (sideways, forward) = if len > 0.0 {
                (dx / len * 0.98, dz / len * 0.98)
            } else {
                (0.0, 0.0)
            };
            out.push(play18::Serverbound::PlayerLook(play18::PlayerLook {
                yaw,
                pitch,
                on_ground,
            }));
            out.push(play18::Serverbound::SteerVehicle(play18::SteerVehicle {
                sideways,
                forward,
                flags: u8::from(input & 0x10 != 0) | u8::from(sneaking) << 1,
            }));
        } else if controls_camera {
            let dx = position[0] - r.legacy.position[0];
            let dy = position[1] - r.legacy.position[1];
            let dz = position[2] - r.legacy.position[2];
            let moved = dx * dx + dy * dy + dz * dz > 9.0E-4 || r.legacy.ticks >= 20;
            let turned = yaw != r.legacy.yaw || pitch != r.legacy.pitch;
            let packet = match (moved, turned) {
                (true, true) => play18::Serverbound::PlayerPositionAndLook(
                    play18::ServerPlayerPositionAndLook {
                        x: position[0],
                        y: position[1],
                        z: position[2],
                        yaw,
                        pitch,
                        on_ground,
                    },
                ),
                (true, false) => play18::Serverbound::PlayerPosition(play18::PlayerPosition {
                    x: position[0],
                    y: position[1],
                    z: position[2],
                    on_ground,
                }),
                (false, true) => play18::Serverbound::PlayerLook(play18::PlayerLook {
                    yaw,
                    pitch,
                    on_ground,
                }),
                (false, false) => play18::Serverbound::Flying(play18::Flying { on_ground }),
            };
            out.push(packet);
            if moved {
                r.legacy.position = position;
                r.legacy.ticks = 0;
            } else {
                r.legacy.ticks += 1;
            }
            r.legacy.yaw = yaw;
            r.legacy.pitch = pitch;
        }
        for packet in out {
            r.session.send_1_8_9(packet);
        }
    }

    /// `NetHandlerPlayClient.handleConfirmTransaction`: accept every server
    /// transaction. 1.8's server holds the window's next click until the
    /// previous one is acknowledged.
    pub(super) fn remote_confirm_transaction_1_8_9(&mut self, window_id: i8, action: i16) {
        if let Some(r) = self.remote.as_ref() {
            r.session.send_1_8_9(play18::Serverbound::ServerTransaction(
                play18::ServerTransaction {
                    window_id,
                    action,
                    accepted: true,
                },
            ));
        }
    }
}

impl Game<'_> {
    /// `C0EPacketClickWindow`: 1.8's click carries the stack on the cursor and
    /// its own transaction id; the server answers with a confirmation this
    /// client accepts (`remote_confirm_transaction_1_8_9`).
    pub(super) fn send_window_click_1_8_9(
        &mut self,
        container: i32,
        slot: i16,
        button: i8,
        mode: i8,
        carried: &Option<horizon_client::items::ItemStack>,
    ) {
        let action = self.next_sequence() as i16;
        let clicked_item = slot_1_8_9(carried);
        if let Some(r) = self.remote.as_ref() {
            r.session
                .send_1_8_9(play18::Serverbound::WindowClick(play18::WindowClick {
                    window_id: container.clamp(0, 255) as u8,
                    slot,
                    mouse_button: button,
                    action,
                    mode,
                    clicked_item,
                }));
        }
    }
}

/// A 1.8 potion effect id as the effect's name.
fn effect_1_8_9(id: i8) -> Option<&'static str> {
    Some(match id {
        1 => "speed",
        2 => "slowness",
        3 => "haste",
        4 => "mining_fatigue",
        5 => "strength",
        6 => "instant_health",
        7 => "instant_damage",
        8 => "jump_boost",
        9 => "nausea",
        10 => "regeneration",
        11 => "resistance",
        12 => "fire_resistance",
        13 => "water_breathing",
        14 => "invisibility",
        15 => "blindness",
        16 => "night_vision",
        17 => "hunger",
        18 => "weakness",
        19 => "poison",
        20 => "wither",
        21 => "health_boost",
        22 => "absorption",
        23 => "saturation",
        _ => return None,
    })
}

impl Game<'_> {
    /// `S2DPacketOpenWindow`: the legacy inventory type as the menu it became,
    /// then the same screen-opening code the current protocol uses (the slot
    /// numbering of every legacy menu matches).
    fn remote_open_window_1_8_9(&mut self, p: &play::OpenWindow) {
        let menu = match p.inventory_type.as_str() {
            "minecraft:chest" => format!("minecraft:generic_9x{}", (p.slot_count / 9).clamp(1, 6)),
            "minecraft:crafting_table" => "minecraft:crafting".into(),
            "minecraft:furnace" => "minecraft:furnace".into(),
            "minecraft:dispenser" | "minecraft:dropper" => "minecraft:generic_3x3".into(),
            "minecraft:enchanting_table" => "minecraft:enchantment".into(),
            "minecraft:brewing_stand" => "minecraft:brewing_stand".into(),
            "minecraft:merchant" => "minecraft:merchant".into(),
            "minecraft:beacon" => "minecraft:beacon".into(),
            "minecraft:anvil" => "minecraft:anvil".into(),
            "minecraft:hopper" => "minecraft:hopper".into(),
            // A horse's window: the packet carries the mount and its slot
            // count (saddle, body, then three per chest column). 1.8 has no
            // MountScreenOpen packet, so the menu is opened from here.
            "EntityHorse" => {
                let columns = usize::from(p.slot_count.saturating_sub(2) / 3).clamp(1, 5);
                if let Some(entity) = p.entity_id
                    && self.open_remote_mount_menu(entity as u32, columns)
                {
                    if let Some(r) = self.remote.as_mut() {
                        r.menu = Some((
                            i32::from(p.window_id),
                            format!("{}{columns}", super::remote::MOUNT_MENU),
                        ));
                        r.menu_title = None;
                    }
                    return;
                }
                self.remote_close_window_1_8_9(p.window_id);
                return;
            }
            other => {
                // A horse's or a mod's window: close it rather than leave the
                // server waiting for clicks this client can't make.
                self.remote_close_window_1_8_9(p.window_id);
                log::debug!("1.8.9: unsupported window type {other}");
                return;
            }
        };
        let Some(menu_id) = self
            .remote
            .as_ref()
            .and_then(|r| r.builtin.get("minecraft:menu")?.id(&menu))
        else {
            self.remote_close_window_1_8_9(p.window_id);
            return;
        };
        let title = horizon_world::nbt::Tag::compound([(
            "text",
            horizon_world::nbt::Tag::String(p.title.as_str().into()),
        )]);
        self.remote_open_screen(current::OpenScreen {
            container: i32::from(p.window_id),
            menu: menu_id,
            title,
        });
    }

    /// `MC|TrList`: a villager's offers, for the merchant window it names.
    ///
    /// 1.8 has no `MerchantOffers` packet: the list is a plugin channel body of
    /// the window id followed by `MerchantRecipeList` (buy, result, the
    /// optional second cost, a disabled flag, the tool uses and their cap).
    fn remote_trade_list_1_8_9(&mut self, data: &[u8]) {
        use horizon_protocol::buf::Codec as _;
        let mut r = horizon_protocol::buf::Reader::new(data);
        let Ok(window) = r.i32() else {
            return;
        };
        if self.open_menu_name(window) != "minecraft:merchant" {
            return;
        }
        let Ok(count) = r.u8() else {
            return;
        };
        let mut offers = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let (Ok(buy), Ok(result)) = (codec18::Slot::read(&mut r), codec18::Slot::read(&mut r))
            else {
                return;
            };
            let (Some(buy), Some(result)) = (item_1_8_9(&buy), item_1_8_9(&result)) else {
                return;
            };
            let second = match r.bool() {
                Ok(false) => None,
                Ok(true) => match codec18::Slot::read(&mut r)
                    .ok()
                    .as_ref()
                    .and_then(item_1_8_9)
                {
                    Some(stack) => Some(stack),
                    None => return,
                },
                Err(_) => return,
            };
            let (Ok(disabled), Ok(uses), Ok(max_uses)) = (r.bool(), r.i32(), r.i32()) else {
                return;
            };
            let mut offer = horizon_client::merchant::MerchantOffer {
                base_cost_a: horizon_client::merchant::ItemCost { stack: buy },
                cost_b: second.map(|stack| horizon_client::merchant::ItemCost { stack }),
                result,
                uses,
                max_uses,
                // 1.8 recipes carry no reward flag, special price or demand.
                reward_exp: true,
                special_price_diff: 0,
                demand: 0,
                price_multiplier: 0.0,
                xp: 0,
            };
            // `compensateToolUses`: a disabled recipe is sold out.
            if disabled {
                offer.uses = offer.max_uses;
            }
            offers.push(offer);
        }
        if let Some(m) = self.station.as_mut().and_then(|s| s.merchant.as_mut()) {
            m.session.offers = offers;
            m.session.villager = true;
        }
    }

    /// `S2EPacketCloseWindow`: the server closes the window it opened.
    fn remote_close_window_1_8_9(&mut self, window_id: u8) {
        if self
            .remote
            .as_ref()
            .and_then(|r| r.menu.as_ref())
            .is_some_and(|(id, _)| *id == i32::from(window_id))
        {
            if let Some(r) = self.remote.as_mut() {
                r.menu = None;
                r.menu_title = None;
            }
            if self.screen == super::Screen::Inventory {
                self.screen = super::Screen::Playing;
                self.capture(true);
            }
        }
    }

    /// `S2FPacketSetSlot`: a window slot, or the cursor at window -1.
    fn remote_set_slot_1_8_9(&mut self, window: i8, slot: i16, item: &codec18::Slot) {
        let stack = horizon_items::legacy_1_8_9::stack(item.id, item.count, item.damage, &item.nbt);
        if let Some(stack) = &stack {
            self.tutorial_get_item(Some(stack));
        }
        if window == -1 {
            self.carried = stack;
            return;
        }
        self.set_menu_slot(i32::from(window), i32::from(slot), stack);
        self.remote_synced();
    }

    /// `S30PacketWindowItems`: every slot of the window at once.
    fn remote_window_items_1_8_9(&mut self, window: u8, items: &[codec18::Slot]) {
        for (slot, item) in items.iter().enumerate() {
            let stack =
                horizon_items::legacy_1_8_9::stack(item.id, item.count, item.damage, &item.nbt);
            self.set_menu_slot(i32::from(window), slot as i32, stack);
        }
        self.remote_synced();
    }

    /// `S1CPacketEntityMetadata` for 1.8's index space: the shared flags, the
    /// name, a living entity's health and a dropped item's stack.
    fn remote_metadata_1_8_9(
        &mut self,
        id: i32,
        metadata: &horizon_protocol::v1_8_9::entity::Metadata,
    ) {
        use horizon_protocol::v1_8_9::entity::MetadataValue;
        // EntityItem.DATA_ITEM: the stack the item entity renders.
        if let Some(MetadataValue::Slot(slot)) = metadata.get(&10)
            && let Some(stack) =
                horizon_items::legacy_1_8_9::stack(slot.id, slot.count, slot.damage, &slot.nbt)
        {
            self.remote_item_stack(id, stack);
        }
        // `EntityItemFrame`: DATA_ITEM 8 (a slot, empty when the frame is
        // empty) and DATA_ROTATION 9. 1.8 keeps the facing in the spawn
        // packet, not in the metadata.
        if let Some(f) = self.item_frames.iter_mut().find(|f| f.id == id as u32) {
            if let Some(MetadataValue::Byte(flags)) = metadata.get(&0) {
                f.invisible = flags & 0x20 != 0;
            }
            if let Some(MetadataValue::Slot(slot)) = metadata.get(&8) {
                f.item =
                    horizon_items::legacy_1_8_9::stack(slot.id, slot.count, slot.damage, &slot.nbt);
            }
            if let Some(MetadataValue::Byte(rotation)) = metadata.get(&9) {
                f.rotation = rotation.rem_euclid(8) as u8;
            }
            return;
        }
        // `EntityArmorStand`: flags 10 and the six pose rotations 11..=16.
        if let Some(a) = self.armor_stands.iter_mut().find(|a| a.id == id as u32) {
            if let Some(MetadataValue::Byte(flags)) = metadata.get(&0) {
                a.invisible = flags & 0x20 != 0;
            }
            if let Some(MetadataValue::String(name)) = metadata.get(&2) {
                let json = serde_json::from_str(name)
                    .unwrap_or_else(|_| serde_json::Value::String(name.clone()));
                a.custom_name = Some(horizon_client::scoreboard::comp_text(&json));
            }
            if let Some(MetadataValue::Byte(visible)) = metadata.get(&3) {
                a.name_visible = *visible != 0;
            }
            if let Some(MetadataValue::Byte(flags)) = metadata.get(&10) {
                a.small = flags & 1 != 0;
                a.show_arms = flags & 4 != 0;
                a.no_base_plate = flags & 8 != 0;
                a.marker = flags & 16 != 0;
            }
            for (slot, index) in (11u8..=16).enumerate() {
                if let Some(MetadataValue::Rotation(pose)) = metadata.get(&index) {
                    a.pose[slot] = *pose;
                }
            }
            return;
        }
        // `EntityArrow.DATA_FLAGS` 16: bit 0 is the critical flag (1.9 moved
        // it to the shared flags at index 8, which the modern path reads).
        if let Some(MetadataValue::Byte(flags)) = metadata.get(&16)
            && let Some(pr) = self
                .creatures
                .projectiles
                .iter_mut()
                .find(|pr| pr.id == id as u32)
        {
            pr.set_critical(flags & 1 != 0);
        }
        let local = self.remote.as_ref().map(|r| r.entity_id);
        let Some(m) = self.mobs.iter_mut().find(|m| m.id == id as u32) else {
            return;
        };
        let species = m.species();
        // Entity.DATA_SHARED_FLAGS_ID (`Entity.getFlag`): on fire 0, crouched
        // 1, sprinting 3, eating 4, invisible 5.
        if let Some(MetadataValue::Byte(flags)) = metadata.get(&0) {
            m.fire_ticks = i32::from(flags & 0x01 != 0);
            m.extra.crouching = flags & 0x02 != 0;
            m.extra.sprinting = flags & 0x08 != 0;
            // `EntityLivingBase.setEating`: bit 4 starts and stops the use.
            if flags & 0x10 == 0 {
                m.extra.using_item = 0;
            } else if m.extra.using_item == 0 {
                m.extra.using_item = 1;
            }
            m.invisible = flags & 0x20 != 0;
        }
        // Entity.DATA_CUSTOM_NAME, DATA_CUSTOM_NAME_VISIBLE and DATA_SILENT.
        if let Some(MetadataValue::String(name)) = metadata.get(&2) {
            let json = serde_json::from_str(name)
                .unwrap_or_else(|_| serde_json::Value::String(name.clone()));
            m.custom_name_text = Some(horizon_client::scoreboard::comp_text(&json));
            m.custom_name = Some(name.clone());
        }
        if let Some(MetadataValue::Byte(visible)) = metadata.get(&3) {
            m.name_visible = *visible != 0;
        }
        if let Some(MetadataValue::Byte(silent)) = metadata.get(&4) {
            m.silent = silent & 1 != 0;
        }
        // LivingEntity.DATA_HEALTH and DATA_ARROW_COUNT.
        if let Some(MetadataValue::Float(health)) = metadata.get(&6) {
            m.health = *health;
        }
        if let Some(MetadataValue::Byte(arrows)) = metadata.get(&9) {
            m.extra.arrow_count = i32::from(*arrows);
        }
        match species {
            // Creeper.DATA_SWELL_DIR (-1 idle), DATA_IS_POWERED, DATA_IS_IGNITED.
            horizon_client::mobs::Species::Creeper => {
                if let Some(MetadataValue::Byte(dir)) = metadata.get(&16) {
                    m.extra.counter = i32::from(*dir);
                }
                if let Some(MetadataValue::Byte(powered)) = metadata.get(&17) {
                    m.extra.charged = *powered == 1;
                }
                if let Some(MetadataValue::Byte(ignited)) = metadata.get(&18) {
                    m.extra.flag = *ignited != 0;
                }
            }
            // Sheep.DATA_WOOL_META: the colour in the low nibble, 16 sheared.
            horizon_client::mobs::Species::Sheep => {
                if let Some(MetadataValue::Byte(wool)) = metadata.get(&16) {
                    m.extra.color = (*wool & 15) as u8;
                    m.extra.sheared = wool & 16 != 0;
                }
            }
            // TameableAnimal.DATA_FLAGS_ID: sitting 1, tamed 4.
            horizon_client::mobs::Species::Wolf
            | horizon_client::mobs::Species::Cat
            | horizon_client::mobs::Species::Parrot
            | horizon_client::mobs::Species::Ocelot => {
                if let Some(MetadataValue::Byte(flags)) = metadata.get(&16) {
                    m.extra.sitting = flags & 1 != 0;
                    m.extra.tame = flags & 4 != 0;
                }
            }
            _ => {}
        }
        match species {
            // Wolf.DATA_COLLAR_COLOR (the tamed collar).
            horizon_client::mobs::Species::Wolf => {
                if let Some(MetadataValue::Byte(color)) = metadata.get(&20) {
                    m.extra.color = (*color & 15) as u8;
                }
            }
            // EntityOcelot.DATA_SKIN_ID: 0 wild, 1..=3 the cat textures.
            horizon_client::mobs::Species::Ocelot => {
                if let Some(MetadataValue::Byte(skin)) = metadata.get(&18) {
                    m.extra.variant = (*skin).clamp(0, 3) as u8;
                }
            }
            // Horse.DATA_TYPE 19 (0 horse, 1 donkey, 2 mule, 3 zombie, 4
            // skeleton) and DATA_ID_TYPE_VARIANT 20 (colour, markings).
            horizon_client::mobs::Species::Horse
            | horizon_client::mobs::Species::Donkey
            | horizon_client::mobs::Species::Mule
            | horizon_client::mobs::Species::ZombieHorse
            | horizon_client::mobs::Species::SkeletonHorse => {
                // DATA_HORSE_FLAGS 16: tamed 2, saddled 4, chested 8.
                if let Some(MetadataValue::Byte(flags)) = metadata.get(&16) {
                    m.extra.tame = flags & 2 != 0;
                    m.set_saddle(
                        (flags & 4 != 0)
                            .then(|| horizon_client::items::ItemStack::new("minecraft:saddle", 1)),
                    );
                    m.extra.chested = flags & 8 != 0;
                }
                if let Some(MetadataValue::Byte(kind)) = metadata.get(&19) {
                    if *kind != 0 {
                        // 1.8 has one entity class for every equine; the
                        // renderer needs its own species (see PORTING.md).
                        log::debug!("1.8.9: horse type {kind} on entity {id}");
                    }
                }
                if let Some(MetadataValue::Int(variant)) = metadata.get(&20) {
                    m.extra.variant = (variant & 0xFF).clamp(0, 6) as u8;
                    m.extra.variant2 = ((variant >> 8) & 0xFF).clamp(0, 4) as u8;
                }
                // DATA_HORSE_ARMOR 22: the renderer keeps the armour's item
                // name in `extra.head` (the equine body layer reads it).
                if let Some(MetadataValue::Slot(slot)) = metadata.get(&22) {
                    m.extra.head = horizon_items::legacy_1_8_9::stack(
                        slot.id,
                        slot.count,
                        slot.damage,
                        &slot.nbt,
                    )
                    .as_ref()
                    .map(|s| std::sync::Arc::from(&*s.item));
                }
            }
            // Villager.DATA_PROFESSION_ID: 0 farmer, 1 librarian, 2 cleric,
            // 3 armorer, 4 butcher.
            horizon_client::mobs::Species::Villager => {
                if let Some(MetadataValue::Int(profession)) = metadata.get(&16) {
                    m.extra.variant2 = legacy_profession(*profession);
                }
            }
            // Rabbit.DATA_TYPE_ID; 99 is the killer bunny, which has no model.
            horizon_client::mobs::Species::Rabbit => {
                if let Some(MetadataValue::Byte(kind)) = metadata.get(&18) {
                    if (0..=5).contains(kind) {
                        m.extra.variant = *kind as u8;
                    }
                }
            }
            // AbstractCubeMob's size.
            horizon_client::mobs::Species::Slime | horizon_client::mobs::Species::MagmaCube => {
                if let Some(MetadataValue::Byte(size)) = metadata.get(&16) {
                    m.extra.size = i32::from(*size).clamp(1, 127);
                }
            }
            // Zombie.DATA_BABY_ID and DATA_CONVERTING_ID.
            horizon_client::mobs::Species::Zombie
            | horizon_client::mobs::Species::ZombieVillager => {
                if let Some(MetadataValue::Byte(baby)) = metadata.get(&12) {
                    m.extra.age = if *baby == 1 { -1 } else { 0 };
                }
                if let Some(MetadataValue::Byte(converting)) = metadata.get(&14) {
                    m.extra.counter2 = i32::from(*converting == 1);
                }
            }
            // Skeleton.DATA_TYPE_ID: 1 is the wither skeleton.
            horizon_client::mobs::Species::Skeleton
            | horizon_client::mobs::Species::WitherSkeleton => {
                if let Some(MetadataValue::Byte(kind)) = metadata.get(&13) {
                    m.extra.variant = (*kind).clamp(0, 1) as u8;
                }
            }
            // Enderman.DATA_CARRY_STATE, DATA_CREEPY and DATA_SCREAMING.
            horizon_client::mobs::Species::Enderman => {
                if let Some(MetadataValue::Short(raw)) = metadata.get(&16) {
                    let legacy = *raw as u16;
                    // `setCarriedBlock` truncates `Block.getByBlockState` to a
                    // short: the block id low 12, the metadata high 4.
                    let block = legacy_1_8_9::block_state_from_packed(i32::from(legacy));
                    m.extra.carried = (legacy != 0)
                        .then(|| self.pack.assets.registry.id_filled(&block))
                        .flatten();
                }
                if let Some(MetadataValue::Byte(creepy)) = metadata.get(&17) {
                    m.extra.flag = *creepy != 0;
                }
                if let Some(MetadataValue::Byte(screaming)) = metadata.get(&18) {
                    m.extra.flag2 = *screaming != 0;
                }
            }
            // Blaze.DATA_FLAGS_ID: burning.
            horizon_client::mobs::Species::Blaze => {
                if let Some(MetadataValue::Byte(flags)) = metadata.get(&16) {
                    m.extra.flag = flags & 1 != 0;
                }
            }
            // Witch.DATA_HEAD_OVERLAY: drinking.
            horizon_client::mobs::Species::Witch => {
                if let Some(MetadataValue::Byte(drinking)) = metadata.get(&21) {
                    m.extra.flag = *drinking == 1;
                }
            }
            // Guardian.DATA_FLAGS_ID (bit 2 retracting spikes) and DATA_TARGET_ID.
            horizon_client::mobs::Species::Guardian
            | horizon_client::mobs::Species::ElderGuardian => {
                if let Some(MetadataValue::Int(flags)) = metadata.get(&16) {
                    m.extra.flag2 = flags & 2 != 0;
                }
                if let Some(MetadataValue::Int(target)) = metadata.get(&17) {
                    m.extra.counter2 = 0;
                    m.extra.flag = *target != 0;
                    m.ai.target = (*target != 0).then_some(if Some(*target) == local {
                        horizon_client::mobs::Who::Player
                    } else {
                        horizon_client::mobs::Who::Mob(*target as u32)
                    });
                }
            }
            // IronGolem.DATA_PLAYER_CREATED: the cracks show.
            horizon_client::mobs::Species::IronGolem => {
                if let Some(MetadataValue::Byte(created)) = metadata.get(&16) {
                    m.extra.flag = created & 1 != 0;
                }
            }
            // WitherBoss: the two side heads' targets (17, 18) and the
            // invulnerable time (20).
            horizon_client::mobs::Species::Wither => {
                if let Some(MetadataValue::Int(target)) = metadata.get(&17) {
                    m.extra.ints[0] = *target;
                }
                if let Some(MetadataValue::Int(target)) = metadata.get(&18) {
                    m.extra.ints[1] = *target;
                }
                if let Some(MetadataValue::Int(ticks)) = metadata.get(&20) {
                    m.extra.counter = (*ticks).max(0);
                }
            }
            // Bat.DATA_FLAGS_ID: resting in bit 0.
            horizon_client::mobs::Species::Bat => {
                if let Some(MetadataValue::Byte(flags)) = metadata.get(&16) {
                    horizon_client::mobs::bat::set_resting(m, flags & 1 != 0);
                }
            }
            _ => {}
        }
        // Player.DATA_ABSORPTION: the golden hearts.
        if let Some(MetadataValue::Float(absorption)) = metadata.get(&17)
            && local == Some(id)
            && let Some(pl) = self.player.as_mut()
        {
            let _ = pl.internal_set_absorption(*absorption);
        }
    }

    /// `S04PacketEntityEquipment`: 1.8 slot 0 held, 1 boots, 2 legs, 3 chest,
    /// 4 helmet — the current order shifted for the four armour slots.
    fn remote_equipment_1_8_9(&mut self, id: i32, slot: i16, item: &codec18::Slot) {
        let stack = horizon_items::legacy_1_8_9::stack(item.id, item.count, item.damage, &item.nbt);
        let name = stack
            .as_ref()
            .map(|s| std::sync::Arc::<str>::from(&*s.item));
        if let Some(a) = self.armor_stands.iter_mut().find(|a| a.id == id as u32) {
            let target = match slot {
                0 => Some(0),
                1 => Some(2),
                2 => Some(3),
                3 => Some(4),
                4 => Some(5),
                _ => None,
            };
            if let Some(i) = target
                && let Some(s) = a.equipment.get_mut(i)
            {
                *s = stack;
            }
            return;
        }
        let Some(m) = self.mobs.iter_mut().find(|m| m.id == id as u32) else {
            return;
        };
        match slot {
            0 => m.extra.set_main_hand_stack(stack),
            1 => {
                m.extra.feet = name;
                m.extra.armor[3] = stack;
            }
            2 => {
                m.extra.legs = name;
                m.extra.armor[2] = stack;
            }
            3 => {
                m.extra.chest = name;
                m.extra.armor[1] = stack;
            }
            4 => {
                m.extra.head = name;
                m.extra.armor[0] = stack;
            }
            _ => {}
        }
    }

    /// `S39PacketPlayerAbilities`: the same flag bits as the current packet.
    fn remote_abilities_1_8_9(&mut self, p: &play::Abilities) {
        let abilities = current::PlayerAbilities {
            flags: p.flags as i8,
            flying_speed: p.flying_speed,
            walking_speed: p.walking_speed,
        };
        self.creative = p.flags & 8 != 0;
        if let Some(r) = self.remote.as_mut() {
            r.abilities = Some(abilities);
        }
        self.apply_remote_abilities();
    }

    /// `S1DPacketEntityEffect`: only this client's own effects are kept, as
    /// the current dispatcher does.
    fn remote_effect_1_8_9(&mut self, p: &play::EntityEffect) {
        if self
            .remote
            .as_ref()
            .is_none_or(|r| r.entity_id != p.entity_id)
        {
            return;
        }
        let Some(id) = effect_1_8_9(p.effect_id) else {
            return;
        };
        let mut e =
            horizon_client::effects::EffectInstance::new(id, p.duration, i32::from(p.amplifier));
        e.visible = !p.hide_particles;
        let list = &mut self.potions.effects.list;
        match list.iter_mut().find(|x| x.id == id) {
            Some(existing) => *existing = e,
            None => list.push(e),
        }
    }

    fn remote_remove_effect_1_8_9(&mut self, p: &play::RemoveEntityEffect) {
        if self
            .remote
            .as_ref()
            .is_none_or(|r| r.entity_id != p.entity_id)
        {
            return;
        }
        if let Some(id) = effect_1_8_9(p.effect_id) {
            self.potions.effects.list.retain(|e| e.id != id);
        }
    }

    /// `S38PacketPlayerListItem`: the tab list's names, modes, latencies,
    /// display names, skins and removals.
    fn remote_player_info_1_8_9(
        &mut self,
        p: &horizon_protocol::v1_8_9::player_info::PlayerInfoPacket,
    ) {
        use horizon_protocol::v1_8_9::player_info as info;
        match p.action {
            info::ADD_PLAYER => {
                for e in &p.entries {
                    let Some(name) = &e.name else { continue };
                    if let Some(r) = self.remote.as_mut() {
                        let player = r.tab_players.entry(e.uuid).or_insert_with(|| {
                            horizon_client::tab_list::TabPlayer::new(e.uuid, name.clone())
                        });
                        player.game_mode = e.game_mode.unwrap_or(0);
                        player.latency = e.latency.unwrap_or(0);
                        player.display_name = e
                            .display_name
                            .as_ref()
                            .and_then(|d| serde_json::from_str::<serde_json::Value>(d).ok());
                        if let Some(t) = e.properties.iter().find(|t| t.name == "textures") {
                            use horizon_client::skins::{self, Signature};
                            let signature = if t.signature.is_some() {
                                Signature::Signed
                            } else {
                                Signature::Unsigned
                            };
                            let textures = skins::unpack(&t.value, signature, &skins::TEXTURE_URIS);
                            r.skin_textures.entry(e.uuid).or_insert(textures);
                        }
                    }
                    if let Some(r) = self.remote.as_mut() {
                        r.players.insert(e.uuid, name.clone());
                    }
                }
            }
            info::UPDATE_GAME_MODE | info::UPDATE_LATENCY | info::UPDATE_DISPLAY_NAME => {
                for e in &p.entries {
                    let Some(r) = self.remote.as_mut() else { break };
                    let Some(t) = r.tab_players.get_mut(&e.uuid) else {
                        continue;
                    };
                    if let Some(mode) = e.game_mode {
                        t.game_mode = mode;
                    }
                    if let Some(latency) = e.latency {
                        t.latency = latency;
                    }
                    if let Some(name) = &e.display_name {
                        t.display_name = serde_json::from_str(name).ok();
                    }
                }
            }
            info::REMOVE_PLAYER => {
                for e in &p.entries {
                    if let Some(r) = self.remote.as_mut() {
                        r.tab_players.remove(&e.uuid);
                        r.skin_textures.remove(&e.uuid);
                        r.listed.remove(&e.uuid);
                    }
                }
            }
            _ => {}
        }
    }

    /// `S47PacketPlayerListHeaderFooter`: JSON components over and under the
    /// tab list.
    fn remote_tab_header_1_8_9(&mut self, p: &play::PlayerListHeader) {
        let text = |s: &String| {
            let json = serde_json::from_str::<serde_json::Value>(s)
                .unwrap_or_else(|_| serde_json::Value::String(s.clone()));
            let text = horizon_client::scoreboard::comp_text(&json);
            text.spans
                .iter()
                .any(|span| !span.text.is_empty())
                .then_some(text)
        };
        self.tab_list.header = text(&p.header);
        self.tab_list.footer = text(&p.footer);
    }

    /// `S45PacketTitle`: 1.8's JSON components into the title overlay.
    fn remote_title_1_8_9(&mut self, p: &play::Title) {
        let text = |s: &String| {
            let json =
                serde_json::from_str(s).unwrap_or_else(|_| serde_json::Value::String(s.clone()));
            horizon_client::scoreboard::comp_text(&json)
        };
        match p {
            play::Title::Text(t) => self.titles.set_title(text(t)),
            play::Title::Subtitle(t) => self.titles.set_subtitle(text(t)),
            play::Title::Times {
                fade_in,
                stay,
                fade_out,
            } => self.titles.set_times(*fade_in, *stay, *fade_out),
            play::Title::Clear => self.titles.clear(),
            play::Title::Reset => {
                self.titles.clear();
                self.titles.reset_times();
            }
        }
    }
}

/// A 1.8 text field as a component: JSON where the field is one, the raw
/// text where it is a plain string (1.8 mixes both between packets).
fn component_1_8_9(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or_else(|_| serde_json::Value::String(text.to_owned()))
}

impl Game<'_> {
    /// `S24PacketBlockAction`: a block's own event. The packet names the
    /// block by its 1.8 id; the event applies only to that block.
    fn remote_block_action_1_8_9(&mut self, p: &play::BlockAction) {
        let pos = p.position;
        let triggered = self.with_level(|b, level, ticks| {
            b.client_block_event(
                level,
                ticks,
                horizon_core::position::BlockPos::new(pos[0], pos[1], pos[2]),
                i32::from(p.byte1),
                i32::from(p.byte2),
            )
        });
        self.sync_moving_pistons();
        let _ = self.process_level_events();
        if triggered {
            return;
        }
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let legacy = self
            .pack
            .assets
            .registry
            .state(state)
            .and_then(legacy_1_8_9::legacy_block_state)
            .is_some_and(|(id, _)| i32::from(id) == p.block_id);
        if legacy {
            self.block_entity_event(pos, i32::from(p.byte1), i32::from(p.byte2));
        }
    }

    /// `S25PacketBlockBreakAnim`: another player's crack on a block.
    fn remote_break_animation_1_8_9(&mut self, p: &play::BlockBreakAnimation) {
        self.destroy_block_progress(p.entity_id, p.position, i32::from(p.destroy_stage));
    }

    /// `S27PacketExplosion`: the blast's particles and sound, and the push
    /// the server computed for this player. The server sends the blocks it
    /// removed separately.
    fn remote_explosion_1_8_9(&mut self, p: &extra18::Explosion) {
        use horizon_client::particles::{ParticleOptions, ParticleType};
        let center = p.position.map(f64::from);
        let pitch =
            (1.0 + (self.world.random.next_float() - self.world.random.next_float()) * 0.2) * 0.7;
        self.audio
            .play("minecraft:entity.generic.explode", Some(center), 4.0, pitch);
        self.particles.add(
            &ParticleOptions::simple(ParticleType::ExplosionEmitter),
            center,
            [1.0, 0.0, 0.0],
        );
        let (world, facts) = (&self.world, self.res.facts());
        self.particles
            .explosion_blocks(center, p.radius, p.affected_blocks.len(), &|b| {
                facts.is_air(world.block(b[0], b[1], b[2]))
            });
        if let Some(player) = self.player.as_mut() {
            player.velocity = Vec3::new(
                player.velocity.x + f64::from(p.player_motion[0]),
                player.velocity.y + f64::from(p.player_motion[1]),
                player.velocity.z + f64::from(p.player_motion[2]),
            );
        }
    }

    /// `S28PacketEffect`: a level event. 1.8's block-break particles carry
    /// the legacy block id, which the current event handler reads as a block
    /// state id.
    fn remote_world_event_1_8_9(&mut self, p: &play::WorldEvent) {
        if p.effect_id == 2001 {
            // `Block.getByBlockState`: the block id in the low 12 bits and the
            // metadata in the top 4, the same packing as the block particles.
            let state = legacy_1_8_9::block_state_from_packed(p.data);
            if let Some(state) = self.pack.assets.registry.id_filled(&state) {
                self.level_event_at(2001, p.position, state.0 as i32);
            }
            return;
        }
        if p.global {
            self.global_level_event_sound(p.effect_id, p.position);
        } else {
            self.level_event_at(p.effect_id, p.position, p.data);
        }
    }

    /// `S29PacketSoundEffect` is already handled; `S2C`'s global entity is a
    /// lightning bolt (1.8's only kind).
    fn remote_global_entity_1_8_9(&mut self, p: &play::SpawnWeather) {
        if p.kind == 1 {
            self.spawn_remote_bolt(
                p.entity_id as u32,
                [f64::from(p.x), f64::from(p.y), f64::from(p.z)],
            );
        }
    }

    /// `S31PacketWindowProperty`: a menu's own numbers (the flame and arrow).
    fn remote_window_property_1_8_9(&mut self, p: &play::WindowProperty) {
        self.remote_container_data(i32::from(p.window_id), p.property, p.value);
    }

    /// `S35PacketUpdateTileEntity`: the 1.8 tag carries its own type name
    /// (`Sign`, `Chest`, ...) and 1.8 field names, which the datafixer turns
    /// into the current block entity NBT.
    fn remote_tile_entity_1_8_9(&mut self, p: &play::TileEntityData) {
        if self.remote_flower_pot_1_8_9(p) {
            return;
        }
        // The skull's type and floor rotation live in its tile entity in 1.8;
        // the engine keeps them in the block state. Rewrite the block here,
        // then let the generic path decode the owner into the block entity.
        self.remote_skull_state_1_8_9(p);
        let Tag::Compound(_) = &p.nbt else {
            return;
        };
        let mut tag = p.nbt.clone();
        // 1.8 tags (numeric `Items`, `Chest`/`Sign` ids, old field names)
        // need every fix from the pre-flattening version, exactly as the chunk
        // converter does; 1631 would skip them.
        horizon_world::datafix::update(
            horizon_world::datafix::schema::Ty::BlockEntity,
            &mut tag,
            -1,
            horizon_world::datafix::CURRENT,
        );
        let Some(Tag::String(name)) = tag.get("id") else {
            return;
        };
        let name = name.to_string_lossy();
        self.remote_block_entity_tag(p.position, &name, &tag);
    }

    /// `SkullBlockEntity`'s update tag: 1.8 has one `minecraft:skull` block
    /// whose `SkullType` and `Rot` come from the tile entity, while the engine
    /// has one block per type and takes the floor rotation from the state.
    ///
    /// Rewrite the block to the typed name, keeping the facing the block
    /// change gave it (a wall skull ignores `Rot`), then let the generic path
    /// decode the owner into the block entity.
    fn remote_skull_state_1_8_9(&mut self, p: &play::TileEntityData) {
        let Some(Tag::String(id)) = p.nbt.get("id") else {
            return;
        };
        let id = id.to_string_lossy();
        if id != "Skull" && id != "minecraft:skull" {
            return;
        }
        let current = self
            .world
            .block(p.position[0], p.position[1], p.position[2]);
        let Some(state) = self.pack.assets.registry.state(current) else {
            return;
        };
        // Not a skull/head yet: the block change has not arrived, so there is
        // nothing to type.
        if horizon_world::legacy_1_8_9::skull_type(&state.name).is_none() {
            return;
        }
        // 1.8's `FACING` is the only block property: `up` is a floor skull
        // (its rotation is the tile entity's `Rot`) and a horizontal facing is
        // a wall skull. Only the six vanilla `SkullType`s exist; the renderer
        // draws anything else as the skeleton skull.
        let wall = state.name.contains("_wall_");
        let skull_type = tag_number(p.nbt.get("SkullType")).unwrap_or(0).clamp(0, 5) as u8;
        let Some(name) = horizon_world::legacy_1_8_9::skull_block_name(skull_type, wall) else {
            return;
        };
        let mut block = state.clone();
        block.name = name.to_owned();
        if !wall {
            // `Rot` is the placed yaw / 22.5 degrees; the state has 16 slots.
            let rot = tag_number(p.nbt.get("Rot")).unwrap_or(0) as u8 & 15;
            block.properties.insert("rotation".into(), rot.to_string());
        }
        let Some(next) = self.pack.assets.registry.id_filled(&block) else {
            return;
        };
        if next != current
            && let Err(e) = self.level_set(p.position, next)
        {
            log::warn!("1.8.9: skull at {:?}: {e}", p.position);
        }
        // The typed block's fresh `SkullBlockEntity` is what the update tag
        // fills. `sync_block_entity` keeps one of the same type, so a repeat
        // update still has a block entity to decode into.
        let key = (p.position[0], p.position[1], p.position[2]);
        self.sync_block_entity(key, Some(current), next);
    }

    /// `TileEntityFlowerPot`: 1.8 keeps the contents in this block entity and
    /// leaves the block as the bare pot. The engine has one block per content,
    /// so turn the tag into that state instead of keeping a block entity.
    /// `S35PacketUpdateTileEntity` sends `Item` as the 1.8 item id; a saved
    /// tag has the item name.
    fn remote_flower_pot_1_8_9(&mut self, p: &play::TileEntityData) -> bool {
        let Tag::Compound(_) = &p.nbt else {
            return false;
        };
        let Some(Tag::String(id)) = p.nbt.get("id") else {
            return false;
        };
        let id = id.to_string_lossy();
        if id != "FlowerPot" && id != "minecraft:flower_pot" {
            return false;
        }
        let item = match p.nbt.get("Item") {
            Some(Tag::String(name)) => name.to_string_lossy(),
            Some(Tag::Int(raw)) => match i16::try_from(*raw)
                .ok()
                .and_then(horizon_items::legacy_1_8_9::legacy_name)
            {
                Some(name) => name.to_owned(),
                None => return true,
            },
            _ => return true,
        };
        let data = match p.nbt.get("Data") {
            Some(Tag::Int(data)) => *data,
            _ => 0,
        };
        let Some(state) = horizon_world::legacy_1_8_9::potted_state(&item, data) else {
            return true;
        };
        let Some(state) = self.pack.assets.registry.id_filled(&state) else {
            return true;
        };
        if let Err(e) = self.level_set(p.position, state) {
            log::warn!("1.8.9: flower pot at {:?}: {e}", p.position);
        }
        self.world
            .block_entities
            .remove(&(p.position[0], p.position[1], p.position[2]));
        true
    }

    /// `S33PacketUpdateSign`: the four lines are JSON components.
    fn remote_update_sign_1_8_9(&mut self, p: &play::UpdateSign) {
        let key = (p.position[0], p.position[1], p.position[2]);
        let lines = [&p.text1, &p.text2, &p.text3, &p.text4];
        if let Some(horizon_client::containers::BlockEntity::Sign(sign)) =
            self.world.block_entities.get_mut(&key)
        {
            for (line, text) in sign.front.lines.iter_mut().zip(lines) {
                line.clone_from(text);
            }
            let state = self.world.block(key.0, key.1, key.2);
            let _ = self.world.set(key.0, key.1, key.2, state);
        }
    }

    /// `S36PacketSignEditorOpen`: 1.8 signs have one side.
    fn remote_open_sign_editor_1_8_9(&mut self, p: &play::OpenSignEditor) {
        self.remote_open_sign_editor(current::OpenSignEditor {
            pos: p.position,
            front: true,
        });
    }

    /// `S43PacketCamera`: the entity the client watches through.
    fn remote_camera_1_8_9(&mut self, p: &play::Camera) {
        self.remote_set_camera(p.entity_id);
    }

    /// `S0APacketUseBed`: only this player's own sleep matters here; other
    /// players' beds are out of sight.
    fn remote_use_bed_1_8_9(&mut self, p: &play::UseBed) {
        if self
            .remote
            .as_ref()
            .is_none_or(|r| r.entity_id != p.entity_id)
        {
            return;
        }
        self.sleeping = Some(p.position);
        self.occupy_bed(p.position);
    }

    /// `S42PacketCombatEvent`: a player died; its message goes to the chat,
    /// as `handleCombatEvent` prints it.
    fn remote_combat_event_1_8_9(&mut self, p: &play::CombatEvent) {
        if let play::CombatEvent::EntityDead { message, .. } = p {
            let text = horizon_client::scoreboard::comp_text(&component_1_8_9(message));
            self.chat_line(text.clone(), horizon_client::chat::ChatSource::Server);
            self.say_system_chat_queued(&text);
        }
    }

    /// `S48PacketResourcePackSend`: this client keeps no server pack, so it
    /// answers with the same DECLINED status a player's "no" sends.
    fn remote_resource_pack_1_8_9(&mut self, p: &play::ResourcePackSend) {
        if let Some(r) = self.remote.as_ref() {
            r.session
                .send_1_8_9(play18::Serverbound::ResourcePackStatus(
                    play18::ResourcePackStatus {
                        hash: p.hash.clone(),
                        result: 1,
                    },
                ));
        }
    }

    /// `S3BPacketScoreboardObjective`: a new objective, a removed one, or
    /// the display name and kind of an existing one.
    fn remote_objective_1_8_9(&mut self, p: &extra18::ScoreboardObjective) {
        use horizon_client::scoreboard::RenderType;
        let render = |kind: &String| {
            if kind == "hearts" {
                RenderType::Hearts
            } else {
                RenderType::Integer
            }
        };
        let board = &mut self.score.board;
        match p {
            extra18::ScoreboardObjective::Create {
                name,
                display_name,
                kind,
            } => {
                board.add_objective(
                    name,
                    "dummy",
                    component_1_8_9(display_name),
                    render(kind),
                    false,
                    None,
                );
            }
            extra18::ScoreboardObjective::Remove { name } => board.remove_objective(name),
            extra18::ScoreboardObjective::Update {
                name,
                display_name,
                kind,
            } => {
                if let Some(o) = board.objective_mut(name) {
                    o.display_name = component_1_8_9(display_name);
                    o.render_type = render(kind);
                }
            }
        }
    }

    /// `S3CPacketUpdateScore`: action 0 sets a score, 1 removes it.
    fn remote_score_1_8_9(&mut self, p: &extra18::ScoreboardScore) {
        let board = &mut self.score.board;
        if board.objective(&p.objective_name).is_none() {
            return;
        }
        match p.action {
            0 => {
                let value = p.value.unwrap_or(0);
                board.score_mut(&p.entity_name, &p.objective_name).value = value;
            }
            1 => board.reset_one(&p.entity_name, &p.objective_name),
            _ => {}
        }
    }

    /// `S3DPacketDisplayScoreboard`: 0 list, 1 sidebar, 2 below the name.
    fn remote_display_objective_1_8_9(&mut self, p: &extra18::DisplayObjective) {
        let slot = match p.position {
            0 => "list".to_owned(),
            2 => "below_name".to_owned(),
            _ => "sidebar".to_owned(),
        };
        let board = &mut self.score.board;
        let objective = (p.name != "dummy" && !p.name.is_empty())
            .then(|| board.objective(&p.name).map(|o| o.name.clone()))
            .flatten();
        board.set_display_objective(&slot, objective.as_deref());
    }

    /// `S3EPacketTeams`: create, remove, update, add players, remove players.
    fn remote_team_1_8_9(&mut self, p: &extra18::ScoreboardTeam) {
        use horizon_client::scoreboard::{CollisionRule, Visibility};
        let board = &mut self.score.board;
        match p {
            extra18::ScoreboardTeam::Remove { team } => {
                board.remove_team(team);
                return;
            }
            extra18::ScoreboardTeam::Create { team, .. }
            | extra18::ScoreboardTeam::Update { team, .. } => {
                if matches!(p, extra18::ScoreboardTeam::Create { .. }) {
                    board.add_team(team);
                }
                let (
                    display_name,
                    prefix,
                    suffix,
                    friendly_fire,
                    visibility,
                    color,
                    players,
                    created,
                ) = match p {
                    extra18::ScoreboardTeam::Create {
                        display_name,
                        prefix,
                        suffix,
                        friendly_fire,
                        name_tag_visibility,
                        color,
                        players,
                        ..
                    } => (
                        display_name,
                        prefix,
                        suffix,
                        friendly_fire,
                        name_tag_visibility,
                        color,
                        Some(players),
                        true,
                    ),
                    extra18::ScoreboardTeam::Update {
                        display_name,
                        prefix,
                        suffix,
                        friendly_fire,
                        name_tag_visibility,
                        color,
                        ..
                    } => (
                        display_name,
                        prefix,
                        suffix,
                        friendly_fire,
                        name_tag_visibility,
                        color,
                        None,
                        false,
                    ),
                    _ => unreachable!(),
                };
                if let Some(t) = board.team_mut(team) {
                    t.display_name = component_1_8_9(display_name);
                    t.prefix = component_1_8_9(prefix);
                    t.suffix = component_1_8_9(suffix);
                    t.allow_friendly_fire = friendly_fire & 1 != 0;
                    t.see_friendly_invisibles = friendly_fire & 2 != 0;
                    t.nametag_visibility = Visibility::ALL
                        .into_iter()
                        .find(|v| v.name() == visibility)
                        .unwrap_or(Visibility::Always);
                    t.collision_rule = CollisionRule::Always;
                    t.color = (0..16).contains(color).then(|| {
                        horizon_client::text::COLORS[usize::from(*color as u8)]
                            .1
                            .to_owned()
                    });
                }
                if created && let Some(players) = players {
                    for player in players {
                        board.add_to_team(player, team);
                    }
                }
            }
            extra18::ScoreboardTeam::AddPlayers { team, players } => {
                for player in players {
                    board.add_to_team(player, team);
                }
            }
            extra18::ScoreboardTeam::RemovePlayers { team, players } => {
                for player in players {
                    if board.players_team(player).is_some_and(|t| t.name == *team) {
                        board.remove_from_team(player);
                    }
                }
            }
        }
    }
}

impl Game<'_> {
    /// `S44PacketWorldBorder`: a size, a resize, the centre, or the warning.
    fn remote_world_border_1_8_9(&mut self, p: &extra18::WorldBorder) {
        match p {
            extra18::WorldBorder::SetSize { radius } => self.border.set_size(*radius),
            extra18::WorldBorder::LerpSize {
                old_radius,
                new_radius,
                speed,
            } => self.border.lerp_between(*old_radius, *new_radius, *speed),
            extra18::WorldBorder::SetCenter { x, z } => self.border.set_center([*x, *z]),
            extra18::WorldBorder::Initialize {
                x,
                z,
                old_radius,
                new_radius,
                speed,
                warning_time,
                warning_blocks,
                ..
            } => {
                self.border.set_center([*x, *z]);
                self.border.lerp_between(*old_radius, *new_radius, *speed);
                self.border.warning_time = *warning_time;
                self.border.warning_distance = *warning_blocks;
            }
            extra18::WorldBorder::SetWarningTime { warning_time } => {
                self.border.warning_time = *warning_time
            }
            extra18::WorldBorder::SetWarningBlocks { warning_blocks } => {
                self.border.warning_distance = *warning_blocks
            }
        }
    }
}
