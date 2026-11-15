//! The play-state packets of 1.8.9 (protocol 47).
//!
//! This covers the joining handshake, the world baseline (time, spawn point,
//! chunks, block changes, health and respawn), entities (spawn, metadata,
//! movement, effects), inventory and windows, scoreboard-adjacent UI packets
//! and the client's movement, digging, placement and settings packets. Packets
//! that are not modelled yet are `Unknown` and keep their raw payload for later
//! stages.
use crate::{
    buf::{
        Bool, Byte, Bytes, Codec, Double, Float, Int, List, Long, NbtClassic, Opt, Reader, Rest,
        Short, Str, StrN, UByte, UShort, Uuid, VarInt, Writer,
    },
    packet, packet_enum,
    v1_8_9::{
        codec::{BlockRecord, BlockRecords, Pos1_8, Slot, Slots},
        entity::{EntityMetadata, Metadata, Velocity},
        play_extra::{
            DisplayObjective, EntityProperties, Explosion, MapUpdate, Particles,
            ScoreboardObjective, ScoreboardScore, ScoreboardTeam, SpawnObject, Statistics,
            UseEntity, WorldBorder,
        },
        player_info::PlayerInfoPacket,
    },
};

packet!(
    /// `PacketPlayOutKeepAlive`.
    KeepAlive { id: i32 => VarInt }
);
packet!(
    /// `PacketPlayOutLogin` (join game). 1.8 has no registry/data-pack payload:
    /// the dimension is a byte and the world type is a short string.
    JoinGame {
        entity_id: i32 => Int,
        gamemode: u8 => UByte,
        dimension: i8 => Byte,
        difficulty: u8 => UByte,
        max_players: u8 => UByte,
        level_type: String => StrN<16>,
        reduced_debug_info: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutChat`.
    Chat {
        message: String => Str,
        position: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayOutUpdateTime`.
    TimeUpdate {
        world_age: i64 => Long,
        time_of_day: i64 => Long,
    }
);
packet!(
    /// `PacketPlayOutSpawnPosition`, a 1.8 packed position.
    SpawnPosition { position: [i32; 3] => Pos1_8 }
);
packet!(
    /// `PacketPlayOutUpdateHealth`.
    UpdateHealth {
        health: f32 => Float,
        food: i32 => VarInt,
        saturation: f32 => Float,
    }
);
packet!(
    /// `PacketPlayOutRespawn`.
    Respawn {
        dimension: i32 => Int,
        difficulty: u8 => UByte,
        gamemode: u8 => UByte,
        level_type: String => StrN<16>,
    }
);
packet!(
    /// `PacketPlayOutPosition`: absolute or relative according to `flags`.
    PlayerPositionAndLook {
        x: f64 => Double,
        y: f64 => Double,
        z: f64 => Double,
        yaw: f32 => Float,
        pitch: f32 => Float,
        flags: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayOutHeldItemChange`.
    HeldItemChange { slot: i8 => Byte }
);
packet!(
    /// `PacketPlayOutMapChunk`: one column, with a VarInt-prefixed compressed
    /// section blob. 1.9 split this into separate chunk packets.
    MapChunk {
        chunk_x: i32 => Int,
        chunk_z: i32 => Int,
        ground_up: bool => Bool,
        primary_bit_mask: u16 => UShort,
        data: Vec<u8> => Bytes,
    }
);
packet!(
    /// `PacketPlayOutMultiBlockChange`: chunk-relative records.
    MultiBlockChange {
        chunk_x: i32 => Int,
        chunk_z: i32 => Int,
        records: Vec<BlockRecord> => BlockRecords,
    }
);
packet!(
    /// `PacketPlayOutBlockChange`: a 1.8 block state (`id << 4 | meta`).
    BlockChange {
        position: [i32; 3] => Pos1_8,
        block_state: i32 => VarInt,
    }
);
/// One column in `PacketPlayOutMapChunkBulk`.
#[derive(Clone, Debug, PartialEq)]
pub struct BulkChunk {
    pub x: i32,
    pub z: i32,
    pub primary_bit_mask: u16,
}
/// `PacketPlayOutMapChunkBulk`: the 1.8 metadata array comes before the
/// concatenated, uncompressed-for-the-wire section data.
#[derive(Clone, Debug, PartialEq)]
pub struct MapChunkBulk {
    pub sky_light_sent: bool,
    pub chunks: Vec<BulkChunk>,
    pub data: Vec<u8>,
}
impl Codec<MapChunkBulk> for MapChunkBulk {
    fn read(r: &mut Reader) -> crate::Result<MapChunkBulk> {
        let sky_light_sent = r.bool()?;
        let count = r.varint()?;
        if count < 0 || count as usize > r.remaining() / 10 {
            return Err(crate::Error::new("invalid map-chunk-bulk count"));
        }
        let chunks = (0..count)
            .map(|_| {
                Ok(BulkChunk {
                    x: r.i32()?,
                    z: r.i32()?,
                    primary_bit_mask: r.u16()?,
                })
            })
            .collect::<crate::Result<Vec<_>>>()?;
        Ok(MapChunkBulk {
            sky_light_sent,
            chunks,
            data: r.rest().to_vec(),
        })
    }
    fn write(v: &MapChunkBulk, w: &mut Writer) {
        w.bool(v.sky_light_sent);
        w.varint(v.chunks.len() as i32);
        for chunk in &v.chunks {
            w.i32(chunk.x);
            w.i32(chunk.z);
            w.u16(chunk.primary_bit_mask);
        }
        w.bytes(&v.data);
    }
}
packet!(
    /// `PacketPlayOutGameStateChange`.
    GameStateChange {
        reason: u8 => UByte,
        value: f32 => Float,
    }
);
packet!(
    /// `PacketPlayOutAbilities`.
    Abilities {
        flags: i8 => Byte,
        flying_speed: f32 => Float,
        walking_speed: f32 => Float,
    }
);
packet!(
    /// `PacketPlayOutKickDisconnect`: a JSON chat component.
    Disconnect { reason_json: String => Str }
);
packet!(
    /// `PacketPlayOutDifficulty`.
    Difficulty { difficulty: u8 => UByte }
);
packet!(
    /// `PacketPlayOutSetCompression` (play state).
    SetCompression { threshold: i32 => VarInt }
);
packet!(
    /// `PacketPlayOutPlayerListHeaderFooter`.
    PlayerListHeader {
        header: String => Str,
        footer: String => Str,
    }
);
packet!(
    /// `PacketPlayOutNamedEntitySpawn`: fixed-point coordinates and metadata.
    SpawnPlayer {
        entity_id: i32 => VarInt,
        uuid: u128 => Uuid,
        x: i32 => Int,
        y: i32 => Int,
        z: i32 => Int,
        yaw: i8 => Byte,
        pitch: i8 => Byte,
        current_item: i16 => Short,
        metadata: Metadata => EntityMetadata,
    }
);
packet!(
    /// `PacketPlayOutSpawnEntityLiving`.
    SpawnMob {
        entity_id: i32 => VarInt,
        kind: u8 => UByte,
        x: i32 => Int,
        y: i32 => Int,
        z: i32 => Int,
        yaw: i8 => Byte,
        pitch: i8 => Byte,
        head_pitch: i8 => Byte,
        velocity: [i16; 3] => Velocity,
        metadata: Metadata => EntityMetadata,
    }
);
packet!(
    /// `PacketPlayOutEntityVelocity`.
    EntityVelocity {
        entity_id: i32 => VarInt,
        velocity: [i16; 3] => Velocity,
    }
);
packet!(
    /// `PacketPlayOutEntityDestroy`.
    DestroyEntities { entity_ids: Vec<i32> => List<VarInt> }
);
packet!(
    /// `PacketPlayOutEntity` (1.8 sends only the entity id).
    Entity { entity_id: i32 => VarInt }
);
packet!(
    /// `PacketPlayOutRelEntityMove`: three 1/32-block deltas.
    EntityRelativeMove {
        entity_id: i32 => VarInt,
        dx: i8 => Byte,
        dy: i8 => Byte,
        dz: i8 => Byte,
        on_ground: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutEntityLook`.
    EntityLook {
        entity_id: i32 => VarInt,
        yaw: i8 => Byte,
        pitch: i8 => Byte,
        on_ground: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutEntityMoveLook`.
    EntityLookAndRelativeMove {
        entity_id: i32 => VarInt,
        dx: i8 => Byte,
        dy: i8 => Byte,
        dz: i8 => Byte,
        yaw: i8 => Byte,
        pitch: i8 => Byte,
        on_ground: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutEntityTeleport`.
    EntityTeleport {
        entity_id: i32 => VarInt,
        x: i32 => Int,
        y: i32 => Int,
        z: i32 => Int,
        yaw: i8 => Byte,
        pitch: i8 => Byte,
        on_ground: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutEntityHeadRotation`.
    EntityHeadLook {
        entity_id: i32 => VarInt,
        head_yaw: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayOutEntityStatus`: the 1.8 entity id is a plain int.
    EntityStatus {
        entity_id: i32 => Int,
        status: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayOutAttachEntity`: both ids are plain ints in 1.8.
    AttachEntity {
        entity_id: i32 => Int,
        vehicle_id: i32 => Int,
        leash: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutEntityMetadata`.
    EntityMetadataPacket {
        entity_id: i32 => VarInt,
        metadata: Metadata => EntityMetadata,
    }
);
packet!(
    /// `PacketPlayOutEntityEffect`.
    EntityEffect {
        entity_id: i32 => VarInt,
        effect_id: i8 => Byte,
        amplifier: i8 => Byte,
        duration: i32 => VarInt,
        hide_particles: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutRemoveEntityEffect`.
    RemoveEntityEffect {
        entity_id: i32 => VarInt,
        effect_id: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayOutSetExperience`.
    SetExperience {
        experience_bar: f32 => Float,
        level: i32 => VarInt,
        total_experience: i32 => VarInt,
    }
);
packet!(
    /// `PacketPlayOutEntityEquipment`: 0 held, 1 boots, 2 leggings, 3 chest,
    /// 4 helmet.
    EntityEquipment {
        entity_id: i32 => VarInt,
        slot: i16 => Short,
        item: Slot => Slot,
    }
);
packet!(
    /// `PacketPlayOutAnimation` (the clientbound arm swing and damage animation).
    EntityAnimation {
        entity_id: i32 => VarInt,
        animation: u8 => UByte,
    }
);
packet!(
    /// `PacketPlayOutBlockAction`.
    BlockAction {
        position: [i32; 3] => Pos1_8,
        byte1: u8 => UByte,
        byte2: u8 => UByte,
        block_id: i32 => VarInt,
    }
);
packet!(
    /// `PacketPlayOutBlockBreakAnimation`.
    BlockBreakAnimation {
        entity_id: i32 => VarInt,
        position: [i32; 3] => Pos1_8,
        destroy_stage: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayOutWorldEvent`.
    WorldEvent {
        effect_id: i32 => Int,
        position: [i32; 3] => Pos1_8,
        data: i32 => Int,
        global: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutNamedSoundEffect`: coordinates are fixed-point `* 8`.
    SoundEffect {
        sound_name: String => Str,
        x: i32 => Int,
        y: i32 => Int,
        z: i32 => Int,
        volume: f32 => Float,
        pitch: u8 => UByte,
    }
);
packet!(
    /// `PacketPlayOutCloseWindow`.
    CloseWindow { window_id: u8 => UByte }
);
packet!(
    /// `PacketPlayOutSetSlot`.
    SetSlot {
        window_id: i8 => Byte,
        slot: i16 => Short,
        item: Slot => Slot,
    }
);
packet!(
    /// `PacketPlayOutWindowItems`: an `i16`-counted slot array.
    WindowItems {
        window_id: u8 => UByte,
        items: Vec<Slot> => Slots,
    }
);
packet!(
    /// `PacketPlayOutWindowProperty`.
    WindowProperty {
        window_id: u8 => UByte,
        property: i16 => Short,
        value: i16 => Short,
    }
);
packet!(
    /// `PacketPlayOutTransaction`.
    ConfirmTransaction {
        window_id: i8 => Byte,
        action: i16 => Short,
        accepted: bool => Bool,
    }
);
packet!(
    /// `PacketPlayOutUpdateSign`.
    UpdateSign {
        position: [i32; 3] => Pos1_8,
        text1: String => Str,
        text2: String => Str,
        text3: String => Str,
        text4: String => Str,
    }
);
packet!(
    /// `PacketPlayOutTileEntityData`: the optional NBT tag (TAG_End = none).
    TileEntityData {
        position: [i32; 3] => Pos1_8,
        action: u8 => UByte,
        nbt: horizon_world::nbt::Tag => NbtClassic,
    }
);
packet!(
    /// `PacketPlayOutOpenSignEditor`.
    OpenSignEditor { position: [i32; 3] => Pos1_8 }
);
packet!(
    /// `PacketPlayOutCustomPayload` (clientbound).
    ClientPluginMessage {
        channel: String => Str,
        data: Vec<u8> => Rest,
    }
);
packet!(
    /// `PacketPlayOutCamera`.
    Camera { entity_id: i32 => VarInt }
);
packet!(
    /// `PacketPlayOutResourcePackSend`.
    ResourcePackSend {
        url: String => Str,
        hash: String => Str,
    }
);
packet!(
    /// `PacketPlayOutUpdateEntityNBT`.
    UpdateEntityNbt {
        entity_id: i32 => VarInt,
        tag: horizon_world::nbt::Tag => NbtClassic,
    }
);

/// `PacketPlayOutOpenWindow`: the entity id is present only for `EntityHorse`.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenWindow {
    pub window_id: u8,
    pub inventory_type: String,
    pub title: String,
    pub slot_count: u8,
    pub entity_id: Option<i32>,
}
impl Codec<OpenWindow> for OpenWindow {
    fn read(r: &mut Reader) -> crate::Result<OpenWindow> {
        let window_id = r.u8()?;
        let inventory_type = r.string(32767)?;
        let title = r.string(32767)?;
        let slot_count = r.u8()?;
        let entity_id = if inventory_type == "EntityHorse" {
            Some(r.i32()?)
        } else {
            None
        };
        Ok(OpenWindow {
            window_id,
            inventory_type,
            title,
            slot_count,
            entity_id,
        })
    }
    fn write(v: &OpenWindow, w: &mut Writer) {
        w.u8(v.window_id);
        w.string(&v.inventory_type);
        w.string(&v.title);
        w.u8(v.slot_count);
        if v.inventory_type == "EntityHorse" {
            w.i32(v.entity_id.unwrap_or(0));
        }
    }
}

/// `PacketPlayOutTitle`: the body depends on the action.
#[derive(Clone, Debug, PartialEq)]
pub enum Title {
    Text(String),
    Subtitle(String),
    Times {
        fade_in: i32,
        stay: i32,
        fade_out: i32,
    },
    Clear,
    Reset,
}
impl Codec<Title> for Title {
    fn read(r: &mut Reader) -> crate::Result<Title> {
        match r.varint()? {
            0 => Ok(Title::Text(r.string(32767)?)),
            1 => Ok(Title::Subtitle(r.string(32767)?)),
            2 => Ok(Title::Times {
                fade_in: r.i32()?,
                stay: r.i32()?,
                fade_out: r.i32()?,
            }),
            3 => Ok(Title::Clear),
            4 => Ok(Title::Reset),
            other => Err(crate::Error::new(format!("unknown title action {other}"))),
        }
    }
    fn write(v: &Title, w: &mut Writer) {
        match v {
            Title::Text(text) => {
                w.varint(0);
                w.string(text);
            }
            Title::Subtitle(text) => {
                w.varint(1);
                w.string(text);
            }
            Title::Times {
                fade_in,
                stay,
                fade_out,
            } => {
                w.varint(2);
                w.i32(*fade_in);
                w.i32(*stay);
                w.i32(*fade_out);
            }
            Title::Clear => w.varint(3),
            Title::Reset => w.varint(4),
        }
    }
}

/// `PacketPlayOutCombatEvent`.
#[derive(Clone, Debug, PartialEq)]
pub enum CombatEvent {
    EnterCombat,
    EndCombat {
        duration: i32,
        entity_id: i32,
    },
    EntityDead {
        player_id: i32,
        entity_id: i32,
        message: String,
    },
}
impl Codec<CombatEvent> for CombatEvent {
    fn read(r: &mut Reader) -> crate::Result<CombatEvent> {
        match r.varint()? {
            0 => Ok(CombatEvent::EnterCombat),
            1 => Ok(CombatEvent::EndCombat {
                duration: r.varint()?,
                entity_id: r.i32()?,
            }),
            2 => Ok(CombatEvent::EntityDead {
                player_id: r.varint()?,
                entity_id: r.i32()?,
                message: r.string(32767)?,
            }),
            other => Err(crate::Error::new(format!("unknown combat event {other}"))),
        }
    }
    fn write(v: &CombatEvent, w: &mut Writer) {
        match v {
            CombatEvent::EnterCombat => w.varint(0),
            CombatEvent::EndCombat {
                duration,
                entity_id,
            } => {
                w.varint(1);
                w.varint(*duration);
                w.i32(*entity_id);
            }
            CombatEvent::EntityDead {
                player_id,
                entity_id,
                message,
            } => {
                w.varint(2);
                w.varint(*player_id);
                w.i32(*entity_id);
                w.string(message);
            }
        }
    }
}

packet!(
    /// `PacketPlayInChat`: no position byte, unlike the clientbound packet.
    /// 1.8.9 caps a chat message at 100 characters.
    ServerChat { message: String => StrN<100> }
);
packet!(
    /// `PacketPlayInFlying` with only the on-ground flag.
    Flying { on_ground: bool => Bool }
);
packet!(
    /// `PacketPlayInPosition`.
    PlayerPosition {
        x: f64 => Double,
        y: f64 => Double,
        z: f64 => Double,
        on_ground: bool => Bool,
    }
);
packet!(
    /// `PacketPlayInLook`.
    PlayerLook {
        yaw: f32 => Float,
        pitch: f32 => Float,
        on_ground: bool => Bool,
    }
);
packet!(
    /// `PacketPlayInPositionLook`.
    ServerPlayerPositionAndLook {
        x: f64 => Double,
        y: f64 => Double,
        z: f64 => Double,
        yaw: f32 => Float,
        pitch: f32 => Float,
        on_ground: bool => Bool,
    }
);
packet!(
    /// `PacketPlayInBlockDig`.
    PlayerDigging {
        status: i32 => VarInt,
        position: [i32; 3] => Pos1_8,
        face: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayInBlockPlace`.
    BlockPlacement {
        position: [i32; 3] => Pos1_8,
        direction: i8 => Byte,
        held_item: Slot => Slot,
        cursor_x: i8 => Byte,
        cursor_y: i8 => Byte,
        cursor_z: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayInHeldItemChange`: a short, unlike the clientbound byte.
    ServerHeldItemChange { slot: i16 => Short }
);
packet!(
    /// `PacketPlayInEntityAction`.
    EntityAction {
        entity_id: i32 => VarInt,
        action: i32 => VarInt,
        jump_boost: i32 => VarInt,
    }
);
packet!(
    /// `PacketPlayInSettings`.
    ClientSettings {
        locale: String => Str,
        view_distance: i8 => Byte,
        chat_flags: i8 => Byte,
        chat_colors: bool => Bool,
        skin_parts: u8 => UByte,
    }
);
packet!(
    /// `PacketPlayInClientCommand`.
    ClientCommand { action: i32 => VarInt }
);
packet!(
    /// `PacketPlayInCustomPayload`: the channel then the whole remaining body.
    PluginMessage {
        channel: String => Str,
        data: Vec<u8> => Rest,
    }
);
packet!(
    /// `PacketPlayInResourcePackStatus`.
    ResourcePackStatus {
        hash: String => Str,
        result: i32 => VarInt,
    }
);

packet!(
    /// `PacketPlayOutUseBed`: the bed position, not an entity position.
    UseBed {
        entity_id: i32 => VarInt,
        position: [i32; 3] => Pos1_8,
    }
);
packet!(
    /// `PacketPlayOutCollectItem`.
    CollectItem {
        collected_entity_id: i32 => VarInt,
        collector_entity_id: i32 => VarInt,
    }
);
packet!(
    /// `PacketPlayOutSpawnEntityPainting`.
    SpawnPainting {
        entity_id: i32 => VarInt,
        title: String => Str,
        position: [i32; 3] => Pos1_8,
        direction: u8 => UByte,
    }
);
packet!(
    /// `PacketPlayOutSpawnEntityExperienceOrb`.
    SpawnExperienceOrb {
        entity_id: i32 => VarInt,
        x: i32 => Int,
        y: i32 => Int,
        z: i32 => Int,
        count: i16 => Short,
    }
);
packet!(
    /// `PacketPlayOutSpawnEntityWeather`.
    SpawnWeather {
        entity_id: i32 => VarInt,
        kind: i8 => Byte,
        x: i32 => Int,
        y: i32 => Int,
        z: i32 => Int,
    }
);
packet!(
    /// `PacketPlayOutTabComplete` (clientbound response).
    TabCompleteResponse { matches: Vec<String> => List<Str> }
);
packet!(
    /// `PacketPlayInArmAnimation`: no body.
    ArmAnimation {}
);
packet!(
    /// `PacketPlayInSteerVehicle`.
    SteerVehicle {
        sideways: f32 => Float,
        forward: f32 => Float,
        flags: u8 => UByte,
    }
);
packet!(
    /// `PacketPlayInCloseWindow`.
    ServerCloseWindow { window_id: u8 => UByte }
);
packet!(
    /// `PacketPlayInWindowClick`: the clicked slot travels with the click.
    WindowClick {
        window_id: u8 => UByte,
        slot: i16 => Short,
        mouse_button: i8 => Byte,
        action: i16 => Short,
        mode: i8 => Byte,
        clicked_item: Slot => Slot,
    }
);
packet!(
    /// `PacketPlayInTransaction`.
    ServerTransaction {
        window_id: i8 => Byte,
        action: i16 => Short,
        accepted: bool => Bool,
    }
);
packet!(
    /// `PacketPlayInSetCreativeSlot`.
    SetCreativeSlot {
        slot: i16 => Short,
        item: Slot => Slot,
    }
);
packet!(
    /// `PacketPlayInEnchantItem`.
    EnchantItem {
        window_id: i8 => Byte,
        enchantment: i8 => Byte,
    }
);
packet!(
    /// `PacketPlayInUpdateSign`.
    ServerUpdateSign {
        position: [i32; 3] => Pos1_8,
        text1: String => Str,
        text2: String => Str,
        text3: String => Str,
        text4: String => Str,
    }
);
packet!(
    /// `PacketPlayInAbilities`.
    ServerAbilities {
        flags: i8 => Byte,
        flying_speed: f32 => Float,
        walking_speed: f32 => Float,
    }
);
packet!(
    /// `PacketPlayInTabComplete`: the optional block position.
    TabCompleteRequest {
        text: String => Str,
        position: Option<[i32; 3]> => Opt<Pos1_8>,
    }
);
packet!(
    /// `PacketPlayInSpectate`.
    Spectate { target: u128 => Uuid }
);

packet_enum!(Clientbound, crate::v1_8_9::ids::play::clientbound {
    KeepAlive(KeepAlive) = KEEP_ALIVE,
    JoinGame(JoinGame) = JOIN_GAME,
    Chat(Chat) = CHAT,
    TimeUpdate(TimeUpdate) = TIME_UPDATE,
    EntityEquipment(EntityEquipment) = ENTITY_EQUIPMENT,
    SpawnPosition(SpawnPosition) = SPAWN_POSITION,
    UpdateHealth(UpdateHealth) = UPDATE_HEALTH,
    Respawn(Respawn) = RESPAWN,
    PlayerPositionAndLook(PlayerPositionAndLook) = PLAYER_POSITION_AND_LOOK,
    HeldItemChange(HeldItemChange) = HELD_ITEM_CHANGE,
    UseBed(UseBed) = USE_BED,
    EntityAnimation(EntityAnimation) = ENTITY_ANIMATION,
    SpawnPlayer(SpawnPlayer) = SPAWN_PLAYER,
    CollectItem(CollectItem) = COLLECT_ITEM,
    SpawnObject(SpawnObject) = SPAWN_OBJECT,
    SpawnMob(SpawnMob) = SPAWN_MOB,
    SpawnPainting(SpawnPainting) = SPAWN_PAINTING,
    SpawnExperienceOrb(SpawnExperienceOrb) = SPAWN_EXPERIENCE_ORB,
    EntityVelocity(EntityVelocity) = ENTITY_VELOCITY,
    DestroyEntities(DestroyEntities) = DESTROY_ENTITIES,
    Entity(Entity) = ENTITY,
    EntityRelativeMove(EntityRelativeMove) = ENTITY_RELATIVE_MOVE,
    EntityLook(EntityLook) = ENTITY_LOOK,
    EntityLookAndRelativeMove(EntityLookAndRelativeMove) = ENTITY_LOOK_AND_RELATIVE_MOVE,
    EntityTeleport(EntityTeleport) = ENTITY_TELEPORT,
    EntityHeadLook(EntityHeadLook) = ENTITY_HEAD_LOOK,
    EntityStatus(EntityStatus) = ENTITY_STATUS,
    AttachEntity(AttachEntity) = ATTACH_ENTITY,
    EntityMetadata(EntityMetadataPacket) = ENTITY_METADATA,
    EntityEffect(EntityEffect) = ENTITY_EFFECT,
    RemoveEntityEffect(RemoveEntityEffect) = REMOVE_ENTITY_EFFECT,
    SetExperience(SetExperience) = SET_EXPERIENCE,
    EntityProperties(EntityProperties) = ENTITY_PROPERTIES,
    MapChunk(MapChunk) = MAP_CHUNK,
    MultiBlockChange(MultiBlockChange) = MULTI_BLOCK_CHANGE,
    BlockChange(BlockChange) = BLOCK_CHANGE,
    BlockAction(BlockAction) = BLOCK_ACTION,
    BlockBreakAnimation(BlockBreakAnimation) = BLOCK_BREAK_ANIMATION,
    MapChunkBulk(MapChunkBulk) = MAP_CHUNK_BULK,
    Explosion(Explosion) = EXPLOSION,
    WorldEvent(WorldEvent) = WORLD_EVENT,
    SoundEffect(SoundEffect) = SOUND_EFFECT,
    Particles(Particles) = WORLD_PARTICLES,
    GameStateChange(GameStateChange) = GAME_STATE_CHANGE,
    SpawnWeather(SpawnWeather) = SPAWN_WEATHER,
    OpenWindow(OpenWindow) = OPEN_WINDOW,
    CloseWindow(CloseWindow) = CLOSE_WINDOW,
    SetSlot(SetSlot) = SET_SLOT,
    WindowItems(WindowItems) = WINDOW_ITEMS,
    WindowProperty(WindowProperty) = WINDOW_PROPERTY,
    ConfirmTransaction(ConfirmTransaction) = CONFIRM_TRANSACTION,
    UpdateSign(UpdateSign) = UPDATE_SIGN,
    MapUpdate(MapUpdate) = MAP,
    TileEntityData(TileEntityData) = TILE_ENTITY_DATA,
    OpenSignEditor(OpenSignEditor) = OPEN_SIGN_EDITOR,
    Statistics(Statistics) = STATISTICS,
    PlayerInfo(PlayerInfoPacket) = PLAYER_INFO,
    Abilities(Abilities) = ABILITIES,
    TabCompleteResponse(TabCompleteResponse) = TAB_COMPLETE,
    ScoreboardObjective(ScoreboardObjective) = SCOREBOARD_OBJECTIVE,
    ScoreboardScore(ScoreboardScore) = SCOREBOARD_SCORE,
    DisplayObjective(DisplayObjective) = DISPLAY_SCOREBOARD,
    ScoreboardTeam(ScoreboardTeam) = SCOREBOARD_TEAM,
    PluginMessage(ClientPluginMessage) = CUSTOM_PAYLOAD,
    Disconnect(Disconnect) = DISCONNECT,
    Difficulty(Difficulty) = DIFFICULTY,
    CombatEvent(CombatEvent) = COMBAT_EVENT,
    Camera(Camera) = CAMERA,
    WorldBorder(WorldBorder) = WORLD_BORDER,
    Title(Title) = TITLE,
    SetCompression(SetCompression) = SET_COMPRESSION,
    PlayerListHeader(PlayerListHeader) = PLAYER_LIST_HEADER,
    ResourcePackSend(ResourcePackSend) = RESOURCE_PACK_SEND,
    UpdateEntityNbt(UpdateEntityNbt) = UPDATE_ENTITY_NBT,
});
packet_enum!(Serverbound, crate::v1_8_9::ids::play::serverbound {
    KeepAlive(KeepAlive) = KEEP_ALIVE,
    Chat(ServerChat) = CHAT,
    UseEntity(UseEntity) = USE_ENTITY,
    Flying(Flying) = FLYING,
    PlayerPosition(PlayerPosition) = PLAYER_POSITION,
    PlayerLook(PlayerLook) = PLAYER_LOOK,
    PlayerPositionAndLook(ServerPlayerPositionAndLook) = PLAYER_POSITION_AND_LOOK,
    PlayerDigging(PlayerDigging) = PLAYER_DIGGING,
    BlockPlacement(BlockPlacement) = BLOCK_PLACEMENT,
    HeldItemChange(ServerHeldItemChange) = HELD_ITEM_CHANGE,
    ArmAnimation(ArmAnimation) = ANIMATION,
    EntityAction(EntityAction) = ENTITY_ACTION,
    SteerVehicle(SteerVehicle) = STEER_VEHICLE,
    ServerCloseWindow(ServerCloseWindow) = CLOSE_WINDOW,
    WindowClick(WindowClick) = WINDOW_CLICK,
    ServerTransaction(ServerTransaction) = TRANSACTION,
    SetCreativeSlot(SetCreativeSlot) = SET_CREATIVE_SLOT,
    EnchantItem(EnchantItem) = ENCHANT_ITEM,
    ServerUpdateSign(ServerUpdateSign) = UPDATE_SIGN,
    ServerAbilities(ServerAbilities) = ABILITIES,
    TabCompleteRequest(TabCompleteRequest) = TAB_COMPLETE,
    ClientSettings(ClientSettings) = CLIENT_SETTINGS,
    ClientCommand(ClientCommand) = CLIENT_COMMAND,
    PluginMessage(PluginMessage) = PLUGIN_MESSAGE,
    Spectate(Spectate) = SPECTATE,
    ResourcePackStatus(ResourcePackStatus) = RESOURCE_PACK_STATUS,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{packets::round_trip, v1_8_9::entity::MetadataValue};

    #[test]
    fn join_game_bytes() {
        let p = Clientbound::JoinGame(JoinGame {
            entity_id: 1,
            gamemode: 0,
            dimension: 0,
            difficulty: 1,
            max_players: 20,
            level_type: "default".into(),
            reduced_debug_info: false,
        });
        assert_eq!(
            p.encode(),
            [
                0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x14, 0x07, b'd', b'e', b'f', b'a',
                b'u', b'l', b't', 0x00,
            ]
        );
        round_trip(p, Clientbound::encode, Clientbound::decode);
    }

    #[test]
    fn keep_alive_is_a_varint() {
        assert_eq!(
            Clientbound::KeepAlive(KeepAlive { id: 256 }).encode(),
            [0x00, 0x80, 0x02]
        );
    }

    #[test]
    fn time_health_and_respawn_bytes() {
        assert_eq!(
            Clientbound::TimeUpdate(TimeUpdate {
                world_age: 1,
                time_of_day: 2,
            })
            .encode(),
            [0x03, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2]
        );
        assert_eq!(
            Clientbound::UpdateHealth(UpdateHealth {
                health: 20.0,
                food: 20,
                saturation: 5.0,
            })
            .encode(),
            [0x06, 0x41, 0xA0, 0, 0, 0x14, 0x40, 0xA0, 0, 0]
        );
        assert_eq!(
            Clientbound::Respawn(Respawn {
                dimension: -1,
                difficulty: 2,
                gamemode: 1,
                level_type: "flat".into(),
            })
            .encode(),
            [
                0x07, 0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0x01, 0x04, b'f', b'l', b'a', b't'
            ]
        );
    }

    #[test]
    fn spawn_position_packs_y_between_x_and_z() {
        let p = Clientbound::SpawnPosition(SpawnPosition {
            position: [1, 2, 3],
        });
        let expected = [0x05, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00, 0x00, 0x03];
        assert_eq!(p.encode(), expected);
        round_trip(p, Clientbound::encode, Clientbound::decode);
    }

    #[test]
    fn map_chunk_and_bulk_layout() {
        let chunk = Clientbound::MapChunk(MapChunk {
            chunk_x: 1,
            chunk_z: -2,
            ground_up: true,
            primary_bit_mask: 1,
            data: vec![1, 2, 3],
        });
        assert_eq!(
            chunk.encode(),
            [
                0x21, 0x00, 0x00, 0x00, 0x01, 0xFF, 0xFF, 0xFF, 0xFE, 0x01, 0x00, 0x01, 0x03, 1, 2,
                3
            ]
        );
        round_trip(chunk, Clientbound::encode, Clientbound::decode);

        let bulk = Clientbound::MapChunkBulk(MapChunkBulk {
            sky_light_sent: true,
            chunks: vec![BulkChunk {
                x: 1,
                z: 2,
                primary_bit_mask: 0x00FF,
            }],
            data: vec![9, 8],
        });
        assert_eq!(
            bulk.encode(),
            [
                0x26, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0xFF, 9, 8
            ]
        );
        round_trip(bulk, Clientbound::encode, Clientbound::decode);
    }

    #[test]
    fn block_change_and_multi_block_change_bytes() {
        assert_eq!(
            Clientbound::BlockChange(BlockChange {
                position: [1, 2, 3],
                block_state: 0x123,
            })
            .encode(),
            [
                0x23, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00, 0x00, 0x03, 0xA3, 0x02
            ]
        );
        let change = Clientbound::MultiBlockChange(MultiBlockChange {
            chunk_x: 1,
            chunk_z: 2,
            records: vec![BlockRecord {
                x: 5,
                z: 9,
                y: 64,
                state: 0x13,
            }],
        });
        assert_eq!(
            change.encode(),
            [0x22, 0, 0, 0, 1, 0, 0, 0, 2, 1, 0x59, 64, 0x13]
        );
        round_trip(change, Clientbound::encode, Clientbound::decode);
    }

    #[test]
    fn world_and_player_baseline_round_trip() {
        for p in [
            Clientbound::HeldItemChange(HeldItemChange { slot: 4 }),
            Clientbound::GameStateChange(GameStateChange {
                reason: 3,
                value: 1.0,
            }),
            Clientbound::Abilities(Abilities {
                flags: 0x0F,
                flying_speed: 0.05,
                walking_speed: 0.1,
            }),
            Clientbound::Difficulty(Difficulty { difficulty: 2 }),
            Clientbound::SetCompression(SetCompression { threshold: 256 }),
            Clientbound::PlayerListHeader(PlayerListHeader {
                header: "{\"text\":\"hi\"}".into(),
                footer: String::new(),
            }),
            Clientbound::PlayerPositionAndLook(PlayerPositionAndLook {
                x: 1.5,
                y: 64.0,
                z: -3.25,
                yaw: 90.0,
                pitch: -10.0,
                flags: 0,
            }),
            Clientbound::Chat(Chat {
                message: "{\"text\":\"hi\"}".into(),
                position: 1,
            }),
            Clientbound::Disconnect(Disconnect {
                reason_json: "{\"text\":\"bye\"}".into(),
            }),
        ] {
            round_trip(p, Clientbound::encode, Clientbound::decode);
        }
    }

    #[test]
    fn entity_spawn_and_metadata_bytes() {
        let spawn = Clientbound::SpawnPlayer(SpawnPlayer {
            entity_id: 1,
            uuid: 0,
            x: 32,
            y: 640,
            z: -32,
            yaw: 0,
            pitch: 0,
            current_item: 0,
            metadata: Metadata::new(),
        });
        let mut expected = vec![0x0C, 0x01];
        expected.extend_from_slice(&[0; 16]);
        expected.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x02, 0x80, 0xFF, 0xFF, 0xFF, 0xE0, 0x00, 0x00,
            0x00, 0x00, 0x7F,
        ]);
        assert_eq!(spawn.encode(), expected);

        let mut metadata = Metadata::new();
        metadata.insert(0, MetadataValue::Byte(2));
        let update = Clientbound::EntityMetadata(EntityMetadataPacket {
            entity_id: 5,
            metadata,
        });
        assert_eq!(update.encode(), [0x1C, 0x05, 0x00, 0x02, 0x7F]);

        let mob = Clientbound::SpawnMob(SpawnMob {
            entity_id: 2,
            kind: 54,
            x: 0,
            y: 0,
            z: 0,
            yaw: 0,
            pitch: 0,
            head_pitch: 0,
            velocity: [0; 3],
            metadata: Metadata::new(),
        });
        let mut expected = vec![0x0F, 0x02, 0x36];
        expected.extend_from_slice(&[0; 21]);
        expected.push(0x7F);
        assert_eq!(mob.encode(), expected);
    }

    #[test]
    fn entity_movement_packets_round_trip() {
        let mut metadata = Metadata::new();
        metadata.insert(1, MetadataValue::Short(300));
        metadata.insert(4, MetadataValue::String("name".into()));
        for p in [
            Clientbound::SpawnPlayer(SpawnPlayer {
                entity_id: 1,
                uuid: 0x1234,
                x: 32,
                y: 640,
                z: -32,
                yaw: 90,
                pitch: -10,
                current_item: 5,
                metadata: metadata.clone(),
            }),
            Clientbound::SpawnMob(SpawnMob {
                entity_id: 2,
                kind: 54,
                x: -32,
                y: 32,
                z: 64,
                yaw: 0,
                pitch: 0,
                head_pitch: 0,
                velocity: [1, -2, 3],
                metadata,
            }),
            Clientbound::EntityVelocity(EntityVelocity {
                entity_id: 3,
                velocity: [4, 5, 6],
            }),
            Clientbound::DestroyEntities(DestroyEntities {
                entity_ids: vec![1, 2, 3],
            }),
            Clientbound::Entity(Entity { entity_id: 4 }),
            Clientbound::EntityRelativeMove(EntityRelativeMove {
                entity_id: 5,
                dx: 1,
                dy: -1,
                dz: 2,
                on_ground: true,
            }),
            Clientbound::EntityLook(EntityLook {
                entity_id: 6,
                yaw: 90,
                pitch: -10,
                on_ground: false,
            }),
            Clientbound::EntityLookAndRelativeMove(EntityLookAndRelativeMove {
                entity_id: 7,
                dx: 1,
                dy: 0,
                dz: -1,
                yaw: 0,
                pitch: 0,
                on_ground: true,
            }),
            Clientbound::EntityTeleport(EntityTeleport {
                entity_id: 8,
                x: 32,
                y: 640,
                z: -32,
                yaw: 90,
                pitch: -10,
                on_ground: true,
            }),
            Clientbound::EntityHeadLook(EntityHeadLook {
                entity_id: 9,
                head_yaw: 90,
            }),
            Clientbound::EntityStatus(EntityStatus {
                entity_id: 10,
                status: 3,
            }),
            Clientbound::AttachEntity(AttachEntity {
                entity_id: 11,
                vehicle_id: 12,
                leash: true,
            }),
            Clientbound::EntityMetadata(EntityMetadataPacket {
                entity_id: 13,
                metadata: Metadata::new(),
            }),
            Clientbound::EntityEffect(EntityEffect {
                entity_id: 14,
                effect_id: 1,
                amplifier: 2,
                duration: 200,
                hide_particles: false,
            }),
            Clientbound::RemoveEntityEffect(RemoveEntityEffect {
                entity_id: 15,
                effect_id: 1,
            }),
            Clientbound::SetExperience(SetExperience {
                experience_bar: 0.5,
                level: 3,
                total_experience: 100,
            }),
            Clientbound::DestroyEntities(DestroyEntities {
                entity_ids: Vec::new(),
            }),
        ] {
            round_trip(p, Clientbound::encode, Clientbound::decode);
        }
    }

    #[test]
    fn equipment_and_window_bytes() {
        assert_eq!(
            Clientbound::EntityEquipment(EntityEquipment {
                entity_id: 1,
                slot: 0,
                item: Slot::new(276, 1, 0),
            })
            .encode(),
            [0x04, 0x01, 0x00, 0x00, 0x01, 0x14, 0x01, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            Clientbound::SetSlot(SetSlot {
                window_id: 0,
                slot: 36,
                item: Slot::empty(),
            })
            .encode(),
            [0x2F, 0x00, 0x00, 0x24, 0xFF, 0xFF]
        );
        assert_eq!(
            Clientbound::WindowItems(WindowItems {
                window_id: 0,
                items: vec![Slot::new(1, 1, 0), Slot::empty()],
            })
            .encode(),
            [
                0x30, 0x00, 0x00, 0x02, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0xFF, 0xFF
            ]
        );
        assert_eq!(
            Clientbound::CloseWindow(CloseWindow { window_id: 2 }).encode(),
            [0x2E, 0x02]
        );
    }

    #[test]
    fn action_and_sound_bytes() {
        assert_eq!(
            Clientbound::EntityAnimation(EntityAnimation {
                entity_id: 1,
                animation: 1,
            })
            .encode(),
            [0x0B, 0x01, 0x01]
        );
        assert_eq!(
            Clientbound::BlockAction(BlockAction {
                position: [1, 2, 3],
                byte1: 1,
                byte2: 0,
                block_id: 54,
            })
            .encode(),
            [
                0x24, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00, 0x00, 0x03, 0x01, 0x00, 0x36
            ]
        );
        assert_eq!(
            Clientbound::SoundEffect(SoundEffect {
                sound_name: "x".into(),
                x: 8,
                y: 0,
                z: -8,
                volume: 1.0,
                pitch: 63,
            })
            .encode(),
            [
                0x29, 0x01, b'x', 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF,
                0xF8, 0x3F, 0x80, 0x00, 0x00, 0x3F
            ]
        );
    }

    #[test]
    fn inventory_and_world_packets_round_trip() {
        for p in [
            Clientbound::EntityEquipment(EntityEquipment {
                entity_id: 1,
                slot: 4,
                item: Slot::new(298, 1, 0),
            }),
            Clientbound::EntityAnimation(EntityAnimation {
                entity_id: 1,
                animation: 2,
            }),
            Clientbound::BlockAction(BlockAction {
                position: [-1, 64, 2],
                byte1: 1,
                byte2: 2,
                block_id: 54,
            }),
            Clientbound::BlockBreakAnimation(BlockBreakAnimation {
                entity_id: 1,
                position: [1, 2, 3],
                destroy_stage: 5,
            }),
            Clientbound::WorldEvent(WorldEvent {
                effect_id: 2001,
                position: [1, 2, 3],
                data: 1,
                global: false,
            }),
            Clientbound::SoundEffect(SoundEffect {
                sound_name: "random.click".into(),
                x: 8,
                y: 16,
                z: -8,
                volume: 0.5,
                pitch: 63,
            }),
            Clientbound::CloseWindow(CloseWindow { window_id: 0 }),
            Clientbound::SetSlot(SetSlot {
                window_id: -1,
                slot: 45,
                item: Slot::new(1, 64, 0),
            }),
            Clientbound::WindowItems(WindowItems {
                window_id: 1,
                items: vec![Slot::new(1, 1, 0), Slot::empty(), Slot::new(276, 1, 0)],
            }),
            Clientbound::WindowProperty(WindowProperty {
                window_id: 2,
                property: 0,
                value: 200,
            }),
            Clientbound::ConfirmTransaction(ConfirmTransaction {
                window_id: 0,
                action: 1,
                accepted: true,
            }),
            Clientbound::UpdateSign(UpdateSign {
                position: [1, 2, 3],
                text1: "{\"text\":\"a\"}".into(),
                text2: String::new(),
                text3: String::new(),
                text4: String::new(),
            }),
            Clientbound::TileEntityData(TileEntityData {
                position: [1, 2, 3],
                action: 1,
                nbt: horizon_world::nbt::Tag::End,
            }),
            Clientbound::OpenSignEditor(OpenSignEditor {
                position: [1, 2, 3],
            }),
            Clientbound::PluginMessage(ClientPluginMessage {
                channel: "MC|Brand".into(),
                data: vec![1, 2],
            }),
            Clientbound::Camera(Camera { entity_id: 7 }),
            Clientbound::ResourcePackSend(ResourcePackSend {
                url: "https://example.invalid/pack.zip".into(),
                hash: "abc".into(),
            }),
            Clientbound::UpdateEntityNbt(UpdateEntityNbt {
                entity_id: 8,
                tag: horizon_world::nbt::Tag::End,
            }),
            Clientbound::PlayerInfo(PlayerInfoPacket {
                action: 4,
                entries: Vec::new(),
            }),
            Clientbound::OpenWindow(OpenWindow {
                window_id: 1,
                inventory_type: "minecraft:chest".into(),
                title: "{\"text\":\"Chest\"}".into(),
                slot_count: 27,
                entity_id: None,
            }),
            Clientbound::OpenWindow(OpenWindow {
                window_id: 2,
                inventory_type: "EntityHorse".into(),
                title: "Horse".into(),
                slot_count: 0,
                entity_id: Some(7),
            }),
            Clientbound::Title(Title::Text("{\"text\":\"Hi\"}".into())),
            Clientbound::Title(Title::Subtitle("sub".into())),
            Clientbound::Title(Title::Times {
                fade_in: 10,
                stay: 70,
                fade_out: 20,
            }),
            Clientbound::Title(Title::Clear),
            Clientbound::Title(Title::Reset),
            Clientbound::CombatEvent(CombatEvent::EnterCombat),
            Clientbound::CombatEvent(CombatEvent::EndCombat {
                duration: 20,
                entity_id: 5,
            }),
            Clientbound::CombatEvent(CombatEvent::EntityDead {
                player_id: 3,
                entity_id: 5,
                message: "{\"text\":\"died\"}".into(),
            }),
        ] {
            round_trip(p, Clientbound::encode, Clientbound::decode);
        }
    }

    #[test]
    fn window_title_and_combat_bytes() {
        assert_eq!(
            Clientbound::OpenWindow(OpenWindow {
                window_id: 1,
                inventory_type: "chest".into(),
                title: "Chest".into(),
                slot_count: 27,
                entity_id: None,
            })
            .encode(),
            [
                0x2D, 0x01, 0x05, b'c', b'h', b'e', b's', b't', 0x05, b'C', b'h', b'e', b's', b't',
                0x1B
            ]
        );
        assert_eq!(
            Clientbound::OpenWindow(OpenWindow {
                window_id: 2,
                inventory_type: "EntityHorse".into(),
                title: "H".into(),
                slot_count: 0,
                entity_id: Some(7),
            })
            .encode(),
            [
                0x2D, 0x02, 0x0B, b'E', b'n', b't', b'i', b't', b'y', b'H', b'o', b'r', b's', b'e',
                0x01, b'H', 0x00, 0x00, 0x00, 0x00, 0x07
            ]
        );
        assert_eq!(
            Clientbound::Title(Title::Text("hi".into())).encode(),
            [0x45, 0x00, 0x02, b'h', b'i']
        );
        assert_eq!(
            Clientbound::Title(Title::Subtitle("hi".into())).encode(),
            [0x45, 0x01, 0x02, b'h', b'i']
        );
        assert_eq!(
            Clientbound::Title(Title::Times {
                fade_in: 10,
                stay: 70,
                fade_out: 20,
            })
            .encode(),
            [0x45, 0x02, 0, 0, 0, 10, 0, 0, 0, 70, 0, 0, 0, 20]
        );
        assert_eq!(Clientbound::Title(Title::Clear).encode(), [0x45, 0x03]);
        assert_eq!(Clientbound::Title(Title::Reset).encode(), [0x45, 0x04]);
        assert_eq!(
            Clientbound::CombatEvent(CombatEvent::EnterCombat).encode(),
            [0x42, 0x00]
        );
        assert_eq!(
            Clientbound::CombatEvent(CombatEvent::EndCombat {
                duration: 20,
                entity_id: 5,
            })
            .encode(),
            [0x42, 0x01, 0x14, 0x00, 0x00, 0x00, 0x05]
        );
        assert_eq!(
            Clientbound::CombatEvent(CombatEvent::EntityDead {
                player_id: 3,
                entity_id: 5,
                message: "x".into(),
            })
            .encode(),
            [0x42, 0x02, 0x03, 0x00, 0x00, 0x00, 0x05, 0x01, b'x']
        );
    }

    #[test]
    fn entity_status_ids_are_plain_ints() {
        assert_eq!(
            Clientbound::EntityStatus(EntityStatus {
                entity_id: 1,
                status: 3,
            })
            .encode(),
            [0x1A, 0x00, 0x00, 0x00, 0x01, 0x03]
        );
        assert_eq!(
            Clientbound::AttachEntity(AttachEntity {
                entity_id: 1,
                vehicle_id: 2,
                leash: true,
            })
            .encode(),
            [0x1B, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x01]
        );
        assert_eq!(
            Clientbound::SetExperience(SetExperience {
                experience_bar: 0.5,
                level: 3,
                total_experience: 100,
            })
            .encode(),
            [0x1F, 0x3F, 0x00, 0x00, 0x00, 0x03, 0x64]
        );
    }

    #[test]
    fn serverbound_movement_bytes() {
        assert_eq!(
            Serverbound::PlayerPosition(PlayerPosition {
                x: 1.0,
                y: 64.0,
                z: -1.0,
                on_ground: true,
            })
            .encode(),
            [
                0x04, 0x3F, 0xF0, 0, 0, 0, 0, 0, 0, 0x40, 0x50, 0, 0, 0, 0, 0, 0, 0xBF, 0xF0, 0, 0,
                0, 0, 0, 0, 0x01
            ]
        );
        assert_eq!(
            Serverbound::ClientSettings(ClientSettings {
                locale: "en_US".into(),
                view_distance: 8,
                chat_flags: 0,
                chat_colors: true,
                skin_parts: 0x7F,
            })
            .encode(),
            [
                0x15, 0x05, b'e', b'n', b'_', b'U', b'S', 0x08, 0x00, 0x01, 0x7F
            ]
        );
        assert_eq!(
            Serverbound::PluginMessage(PluginMessage {
                channel: "MC|Brand".into(),
                data: vec![1, 2],
            })
            .encode(),
            [
                0x17, 0x08, b'M', b'C', b'|', b'B', b'r', b'a', b'n', b'd', 1, 2
            ]
        );
    }

    #[test]
    fn serverbound_digging_and_placement_bytes() {
        assert_eq!(
            Serverbound::PlayerDigging(PlayerDigging {
                status: 0,
                position: [1, 2, 3],
                face: 1,
            })
            .encode(),
            [
                0x07, 0x00, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00, 0x00, 0x03, 0x01
            ]
        );
        assert_eq!(
            Serverbound::BlockPlacement(BlockPlacement {
                position: [1, 2, 3],
                direction: 1,
                held_item: Slot::new(1, 1, 0),
                cursor_x: 8,
                cursor_y: 8,
                cursor_z: 8,
            })
            .encode(),
            [
                0x08, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00, 0x00, 0x03, 0x01, 0x00, 0x01, 0x01, 0x00,
                0x00, 0x00, 8, 8, 8
            ]
        );
        assert_eq!(
            Serverbound::HeldItemChange(ServerHeldItemChange { slot: 4 }).encode(),
            [0x09, 0x00, 0x04]
        );
    }

    #[test]
    fn remaining_serverbound_packets_round_trip() {
        for p in [
            Serverbound::KeepAlive(KeepAlive { id: 7 }),
            Serverbound::Chat(ServerChat {
                message: "hello".into(),
            }),
            Serverbound::Flying(Flying { on_ground: false }),
            Serverbound::PlayerLook(PlayerLook {
                yaw: 90.0,
                pitch: -10.0,
                on_ground: true,
            }),
            Serverbound::PlayerPositionAndLook(ServerPlayerPositionAndLook {
                x: 0.5,
                y: 64.0,
                z: 0.5,
                yaw: 1.0,
                pitch: 2.0,
                on_ground: true,
            }),
            Serverbound::EntityAction(EntityAction {
                entity_id: 1,
                action: 0,
                jump_boost: 0,
            }),
            Serverbound::ClientCommand(ClientCommand { action: 0 }),
            Serverbound::ResourcePackStatus(ResourcePackStatus {
                hash: "abc".into(),
                result: 2,
            }),
            Serverbound::UseEntity(UseEntity::Interact { target: 1 }),
            Serverbound::UseEntity(UseEntity::Attack { target: 1 }),
            Serverbound::UseEntity(UseEntity::InteractAt {
                target: 1,
                x: 0.5,
                y: 1.0,
                z: -0.5,
            }),
            Serverbound::ArmAnimation(ArmAnimation {}),
            Serverbound::SteerVehicle(SteerVehicle {
                sideways: 0.5,
                forward: -1.0,
                flags: 3,
            }),
            Serverbound::ServerCloseWindow(ServerCloseWindow { window_id: 1 }),
            Serverbound::WindowClick(WindowClick {
                window_id: 0,
                slot: 36,
                mouse_button: 0,
                action: 1,
                mode: 0,
                clicked_item: Slot::new(276, 1, 0),
            }),
            Serverbound::ServerTransaction(ServerTransaction {
                window_id: 0,
                action: 1,
                accepted: true,
            }),
            Serverbound::SetCreativeSlot(SetCreativeSlot {
                slot: 36,
                item: Slot::empty(),
            }),
            Serverbound::EnchantItem(EnchantItem {
                window_id: 1,
                enchantment: 2,
            }),
            Serverbound::ServerUpdateSign(ServerUpdateSign {
                position: [1, 2, 3],
                text1: "a".into(),
                text2: String::new(),
                text3: String::new(),
                text4: String::new(),
            }),
            Serverbound::ServerAbilities(ServerAbilities {
                flags: 1,
                flying_speed: 0.05,
                walking_speed: 0.1,
            }),
            Serverbound::TabCompleteRequest(TabCompleteRequest {
                text: "/tp".into(),
                position: Some([1, 2, 3]),
            }),
            Serverbound::TabCompleteRequest(TabCompleteRequest {
                text: "/tp".into(),
                position: None,
            }),
            Serverbound::Spectate(Spectate { target: 5 }),
        ] {
            round_trip(p, Serverbound::encode, Serverbound::decode);
        }
    }

    #[test]
    fn remaining_clientbound_packets_round_trip() {
        for p in [
            Clientbound::UseBed(UseBed {
                entity_id: 1,
                position: [1, 2, 3],
            }),
            Clientbound::CollectItem(CollectItem {
                collected_entity_id: 1,
                collector_entity_id: 2,
            }),
            Clientbound::SpawnObject(SpawnObject {
                entity_id: 3,
                kind: 1,
                position: [32, 640, -32],
                pitch: 0,
                yaw: 90,
                object_data: 0,
                velocity: None,
            }),
            Clientbound::SpawnObject(SpawnObject {
                entity_id: 4,
                kind: 60,
                position: [0, 0, 0],
                pitch: 0,
                yaw: 0,
                object_data: 1,
                velocity: Some([1, -2, 3]),
            }),
            Clientbound::SpawnPainting(SpawnPainting {
                entity_id: 5,
                title: "Kebab".into(),
                position: [1, 2, 3],
                direction: 2,
            }),
            Clientbound::SpawnExperienceOrb(SpawnExperienceOrb {
                entity_id: 6,
                x: 0,
                y: 0,
                z: 0,
                count: 3,
            }),
            Clientbound::SpawnWeather(SpawnWeather {
                entity_id: 7,
                kind: 1,
                x: 0,
                y: 0,
                z: 0,
            }),
            Clientbound::EntityProperties(EntityProperties {
                entity_id: 8,
                properties: Vec::new(),
            }),
            Clientbound::Explosion(Explosion {
                position: [1.0, 2.0, 3.0],
                radius: 4.0,
                affected_blocks: vec![[1, 2, 3]],
                player_motion: [0.0; 3],
            }),
            Clientbound::Particles(Particles {
                particle_id: 0,
                long_distance: false,
                position: [1.0, 2.0, 3.0],
                offset: [0.0; 3],
                speed: 0.1,
                count: 1,
                data: Vec::new(),
            }),
            Clientbound::MapUpdate(MapUpdate {
                item_damage: 1,
                scale: 0,
                icons: Vec::new(),
                patch: None,
            }),
            Clientbound::Statistics(Statistics(Vec::new())),
            Clientbound::TabCompleteResponse(TabCompleteResponse {
                matches: vec!["a".into(), "b".into()],
            }),
            Clientbound::ScoreboardObjective(ScoreboardObjective::Remove { name: "obj".into() }),
            Clientbound::ScoreboardScore(ScoreboardScore {
                entity_name: "p".into(),
                action: 0,
                objective_name: "obj".into(),
                value: Some(1),
            }),
            Clientbound::DisplayObjective(DisplayObjective {
                position: 1,
                name: "obj".into(),
            }),
            Clientbound::ScoreboardTeam(ScoreboardTeam::Remove {
                team: "team".into(),
            }),
            Clientbound::WorldBorder(WorldBorder::SetCenter { x: 1.0, z: 2.0 }),
        ] {
            round_trip(p, Clientbound::encode, Clientbound::decode);
        }
    }

    #[test]
    fn optional_fields_follow_their_discriminators() {
        let bed = Clientbound::UseBed(UseBed {
            entity_id: 1,
            position: [1, 2, 3],
        });
        assert_eq!(
            bed.encode(),
            [0x0A, 0x01, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00, 0x00, 0x03]
        );
        let still = Clientbound::SpawnObject(SpawnObject {
            entity_id: 1,
            kind: 1,
            position: [0, 0, 0],
            pitch: 0,
            yaw: 0,
            object_data: 0,
            velocity: Some([1, 2, 3]),
        });
        assert_eq!(still.encode().len(), 2 + 1 + 12 + 2 + 4);
        let moving = Clientbound::SpawnObject(SpawnObject {
            entity_id: 1,
            kind: 1,
            position: [0, 0, 0],
            pitch: 0,
            yaw: 0,
            object_data: 1,
            velocity: Some([1, 2, 3]),
        });
        assert_eq!(moving.encode().len(), 2 + 1 + 12 + 2 + 4 + 6);
        assert_eq!(Serverbound::ArmAnimation(ArmAnimation {}).encode(), [0x0A]);
        assert_eq!(
            Serverbound::TabCompleteRequest(TabCompleteRequest {
                text: "x".into(),
                position: None,
            })
            .encode(),
            [0x14, 0x01, b'x', 0x00]
        );
        assert_eq!(
            Serverbound::TabCompleteRequest(TabCompleteRequest {
                text: "x".into(),
                position: Some([1, 2, 3]),
            })
            .encode(),
            [
                0x14, 0x01, b'x', 0x01, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00, 0x00, 0x03
            ]
        );
    }

    #[test]
    fn serverbound_chat_has_no_position_byte() {
        assert_eq!(
            Serverbound::Chat(ServerChat {
                message: "hi".into(),
            })
            .encode(),
            [0x01, 0x02, b'h', b'i']
        );
    }

    #[test]
    fn an_unmodeled_id_keeps_its_payload() {
        assert_eq!(
            Clientbound::decode(0x7F, &[1, 2, 3]).unwrap(),
            Clientbound::Unknown(0x7F, vec![1, 2, 3])
        );
    }
}
