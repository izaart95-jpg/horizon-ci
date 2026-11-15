//! Play packets whose body depends on a discriminator, kept out of `play.rs`.
//!
//! These are the 1.8.9 packets that a flat `packet!` tuple cannot describe:
//! scoreboard actions, teams, the world border, map updates, statistics,
//! particles, explosions, attributes and the serverbound use-entity action.
//! Each codec reads the discriminator first; fields that only exist for some
//! actions are `Option`/enum variants so a write cannot invent them either.
use crate::{
    Result,
    buf::{Codec, Reader, Writer},
};

/// `PacketPlayOutUpdateAttributes`: one entity and its attribute list.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityProperties {
    pub entity_id: i32,
    pub properties: Vec<Property>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub key: String,
    pub value: f64,
    pub modifiers: Vec<Modifier>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Modifier {
    pub uuid: u128,
    pub amount: f64,
    pub operation: i8,
}
impl Codec<EntityProperties> for EntityProperties {
    fn read(r: &mut Reader) -> Result<EntityProperties> {
        let entity_id = r.varint()?;
        let count = r.i32()?;
        if count < 0 || count as usize > r.remaining() / 2 {
            return Err(crate::Error::new("invalid attribute count"));
        }
        let mut properties = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let key = r.string(32767)?;
            let value = r.f64()?;
            let modifiers = r.varint()?;
            if modifiers < 0 || modifiers as usize > r.remaining() / 2 {
                return Err(crate::Error::new("invalid modifier count"));
            }
            let mut list = Vec::with_capacity(modifiers as usize);
            for _ in 0..modifiers {
                list.push(Modifier {
                    uuid: r.uuid()?,
                    amount: r.f64()?,
                    operation: r.i8()?,
                });
            }
            properties.push(Property {
                key,
                value,
                modifiers: list,
            });
        }
        Ok(EntityProperties {
            entity_id,
            properties,
        })
    }
    fn write(v: &EntityProperties, w: &mut Writer) {
        w.varint(v.entity_id);
        w.i32(v.properties.len() as i32);
        for property in &v.properties {
            w.string(&property.key);
            w.f64(property.value);
            w.varint(property.modifiers.len() as i32);
            for modifier in &property.modifiers {
                w.uuid(modifier.uuid);
                w.f64(modifier.amount);
                w.i8(modifier.operation);
            }
        }
    }
}

/// `PacketPlayOutExplosion`: block offsets are three signed bytes each.
#[derive(Clone, Debug, PartialEq)]
pub struct Explosion {
    pub position: [f32; 3],
    pub radius: f32,
    pub affected_blocks: Vec<[i8; 3]>,
    pub player_motion: [f32; 3],
}
impl Codec<Explosion> for Explosion {
    fn read(r: &mut Reader) -> Result<Explosion> {
        let position = [r.f32()?, r.f32()?, r.f32()?];
        let radius = r.f32()?;
        let count = r.i32()?;
        if count < 0 || count as usize > r.remaining() / 3 {
            return Err(crate::Error::new("invalid explosion block count"));
        }
        let mut affected_blocks = Vec::with_capacity(count as usize);
        for _ in 0..count {
            affected_blocks.push([r.i8()?, r.i8()?, r.i8()?]);
        }
        Ok(Explosion {
            position,
            radius,
            affected_blocks,
            player_motion: [r.f32()?, r.f32()?, r.f32()?],
        })
    }
    fn write(v: &Explosion, w: &mut Writer) {
        for c in v.position {
            w.f32(c);
        }
        w.f32(v.radius);
        w.i32(v.affected_blocks.len() as i32);
        for offset in &v.affected_blocks {
            for c in offset {
                w.i8(*c);
            }
        }
        for c in v.player_motion {
            w.f32(c);
        }
    }
}

/// How many trailing VarInts a 1.8 particle id carries.
pub fn particle_data_len(particle_id: i32) -> usize {
    match particle_id {
        36 => 2,
        37 | 38 => 1,
        _ => 0,
    }
}

/// `PacketPlayOutWorldParticles`.
#[derive(Clone, Debug, PartialEq)]
pub struct Particles {
    pub particle_id: i32,
    pub long_distance: bool,
    pub position: [f32; 3],
    pub offset: [f32; 3],
    pub speed: f32,
    pub count: i32,
    /// Trailing block id/metadata, present only for ids 36, 37 and 38.
    pub data: Vec<i32>,
}
impl Codec<Particles> for Particles {
    fn read(r: &mut Reader) -> Result<Particles> {
        let particle_id = r.i32()?;
        let long_distance = r.bool()?;
        let position = [r.f32()?, r.f32()?, r.f32()?];
        let offset = [r.f32()?, r.f32()?, r.f32()?];
        let speed = r.f32()?;
        let count = r.i32()?;
        let data = (0..particle_data_len(particle_id))
            .map(|_| r.varint())
            .collect::<Result<Vec<_>>>()?;
        Ok(Particles {
            particle_id,
            long_distance,
            position,
            offset,
            speed,
            count,
            data,
        })
    }
    fn write(v: &Particles, w: &mut Writer) {
        w.i32(v.particle_id);
        w.bool(v.long_distance);
        for c in v.position {
            w.f32(c);
        }
        for c in v.offset {
            w.f32(c);
        }
        w.f32(v.speed);
        w.i32(v.count);
        // Write exactly what the id expects, so a wrong `data` length cannot
        // desync the stream.
        for i in 0..particle_data_len(v.particle_id) {
            w.varint(v.data.get(i).copied().unwrap_or(0));
        }
    }
}

/// `PacketPlayOutSpawnEntity`: `object_data == 0` omits the velocity.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnObject {
    pub entity_id: i32,
    pub kind: i8,
    pub position: [i32; 3],
    pub pitch: i8,
    pub yaw: i8,
    pub object_data: i32,
    pub velocity: Option<[i16; 3]>,
}
impl Codec<SpawnObject> for SpawnObject {
    fn read(r: &mut Reader) -> Result<SpawnObject> {
        let entity_id = r.varint()?;
        let kind = r.i8()?;
        let position = [r.i32()?, r.i32()?, r.i32()?];
        let pitch = r.i8()?;
        let yaw = r.i8()?;
        let object_data = r.i32()?;
        let velocity = if object_data != 0 {
            Some([r.i16()?, r.i16()?, r.i16()?])
        } else {
            None
        };
        Ok(SpawnObject {
            entity_id,
            kind,
            position,
            pitch,
            yaw,
            object_data,
            velocity,
        })
    }
    fn write(v: &SpawnObject, w: &mut Writer) {
        w.varint(v.entity_id);
        w.i8(v.kind);
        for c in v.position {
            w.i32(c);
        }
        w.i8(v.pitch);
        w.i8(v.yaw);
        w.i32(v.object_data);
        if v.object_data != 0 {
            let velocity = v.velocity.unwrap_or([0; 3]);
            for c in velocity {
                w.i16(c);
            }
        }
    }
}

/// One map icon.
#[derive(Clone, Debug, PartialEq)]
pub struct MapIcon {
    pub direction_and_type: i8,
    pub x: i8,
    pub z: i8,
}
/// The patch carried by a map update; absent when `columns == 0`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPatch {
    pub columns: i8,
    pub rows: i8,
    pub x: i8,
    pub y: i8,
    pub data: Vec<u8>,
}
/// `PacketPlayOutMap`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapUpdate {
    pub item_damage: i32,
    pub scale: i8,
    pub icons: Vec<MapIcon>,
    pub patch: Option<MapPatch>,
}
impl Codec<MapUpdate> for MapUpdate {
    fn read(r: &mut Reader) -> Result<MapUpdate> {
        let item_damage = r.varint()?;
        let scale = r.i8()?;
        let count = r.varint()?;
        if count < 0 || count as usize > r.remaining() / 3 {
            return Err(crate::Error::new("invalid map icon count"));
        }
        let mut icons = Vec::with_capacity(count as usize);
        for _ in 0..count {
            icons.push(MapIcon {
                direction_and_type: r.i8()?,
                x: r.i8()?,
                z: r.i8()?,
            });
        }
        let columns = r.i8()?;
        let patch = if columns == 0 {
            None
        } else {
            let rows = r.i8()?;
            let x = r.i8()?;
            let y = r.i8()?;
            let len = r.varint()?;
            if len < 0 || len as usize > r.remaining() {
                return Err(crate::Error::new("invalid map column count"));
            }
            Some(MapPatch {
                columns,
                rows,
                x,
                y,
                data: r.bytes(len as usize)?.to_vec(),
            })
        };
        Ok(MapUpdate {
            item_damage,
            scale,
            icons,
            patch,
        })
    }
    fn write(v: &MapUpdate, w: &mut Writer) {
        w.varint(v.item_damage);
        w.i8(v.scale);
        w.varint(v.icons.len() as i32);
        for icon in &v.icons {
            w.i8(icon.direction_and_type);
            w.i8(icon.x);
            w.i8(icon.z);
        }
        match &v.patch {
            Some(patch) if patch.columns != 0 => {
                w.i8(patch.columns);
                w.i8(patch.rows);
                w.i8(patch.x);
                w.i8(patch.y);
                w.varint(patch.data.len() as i32);
                w.bytes(&patch.data);
            }
            _ => w.i8(0),
        }
    }
}

/// `PacketPlayOutStatistics`: `(statistic name, value)` pairs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Statistics(pub Vec<(String, i32)>);
impl Codec<Statistics> for Statistics {
    fn read(r: &mut Reader) -> Result<Statistics> {
        let count = r.varint()?;
        if count < 0 || count as usize > r.remaining() / 2 {
            return Err(crate::Error::new("invalid statistic count"));
        }
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            entries.push((r.string(32767)?, r.varint()?));
        }
        Ok(Statistics(entries))
    }
    fn write(v: &Statistics, w: &mut Writer) {
        w.varint(v.0.len() as i32);
        for (name, value) in &v.0 {
            w.string(name);
            w.varint(*value);
        }
    }
}

/// `PacketPlayOutScoreboardObjective`: action 0 create, 1 remove, 2 update.
#[derive(Clone, Debug, PartialEq)]
pub enum ScoreboardObjective {
    Create {
        name: String,
        display_name: String,
        kind: String,
    },
    Remove {
        name: String,
    },
    Update {
        name: String,
        display_name: String,
        kind: String,
    },
}
impl Codec<ScoreboardObjective> for ScoreboardObjective {
    fn read(r: &mut Reader) -> Result<ScoreboardObjective> {
        let name = r.string(16)?;
        match r.i8()? {
            0 => Ok(ScoreboardObjective::Create {
                name,
                display_name: r.string(32)?,
                kind: r.string(16)?,
            }),
            1 => Ok(ScoreboardObjective::Remove { name }),
            2 => Ok(ScoreboardObjective::Update {
                name,
                display_name: r.string(32)?,
                kind: r.string(16)?,
            }),
            other => Err(crate::Error::new(format!(
                "unknown scoreboard objective action {other}"
            ))),
        }
    }
    fn write(v: &ScoreboardObjective, w: &mut Writer) {
        match v {
            ScoreboardObjective::Create {
                name,
                display_name,
                kind,
            } => {
                w.string(name);
                w.i8(0);
                w.string(display_name);
                w.string(kind);
            }
            ScoreboardObjective::Remove { name } => {
                w.string(name);
                w.i8(1);
            }
            ScoreboardObjective::Update {
                name,
                display_name,
                kind,
            } => {
                w.string(name);
                w.i8(2);
                w.string(display_name);
                w.string(kind);
            }
        }
    }
}

/// `PacketPlayOutScoreboardScore`: action 0 create/update, 1 remove.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreboardScore {
    pub entity_name: String,
    pub action: i32,
    pub objective_name: String,
    /// Present unless the action is 1 (remove).
    pub value: Option<i32>,
}
impl Codec<ScoreboardScore> for ScoreboardScore {
    fn read(r: &mut Reader) -> Result<ScoreboardScore> {
        let entity_name = r.string(40)?;
        let action = r.varint()?;
        let objective_name = r.string(16)?;
        let value = if action == 1 { None } else { Some(r.varint()?) };
        Ok(ScoreboardScore {
            entity_name,
            action,
            objective_name,
            value,
        })
    }
    fn write(v: &ScoreboardScore, w: &mut Writer) {
        w.string(&v.entity_name);
        w.varint(v.action);
        w.string(&v.objective_name);
        if v.action != 1 {
            w.varint(v.value.unwrap_or(0));
        }
    }
}

/// `PacketPlayOutScoreboardDisplayObjective`.
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayObjective {
    pub position: i8,
    pub name: String,
}
impl Codec<DisplayObjective> for DisplayObjective {
    fn read(r: &mut Reader) -> Result<DisplayObjective> {
        Ok(DisplayObjective {
            position: r.i8()?,
            name: r.string(16)?,
        })
    }
    fn write(v: &DisplayObjective, w: &mut Writer) {
        w.i8(v.position);
        w.string(&v.name);
    }
}

/// `PacketPlayOutScoreboardTeam`: modes 0 create, 1 remove, 2 update,
/// 3 add players, 4 remove players.
#[derive(Clone, Debug, PartialEq)]
pub enum ScoreboardTeam {
    Create {
        team: String,
        display_name: String,
        prefix: String,
        suffix: String,
        friendly_fire: i8,
        name_tag_visibility: String,
        color: i8,
        players: Vec<String>,
    },
    Remove {
        team: String,
    },
    Update {
        team: String,
        display_name: String,
        prefix: String,
        suffix: String,
        friendly_fire: i8,
        name_tag_visibility: String,
        color: i8,
    },
    AddPlayers {
        team: String,
        players: Vec<String>,
    },
    RemovePlayers {
        team: String,
        players: Vec<String>,
    },
}
fn read_players(r: &mut Reader) -> Result<Vec<String>> {
    let count = r.varint()?;
    if count < 0 || count as usize > r.remaining() / 2 {
        return Err(crate::Error::new("invalid team player count"));
    }
    (0..count).map(|_| r.string(40)).collect()
}
fn write_players(players: &[String], w: &mut Writer) {
    w.varint(players.len() as i32);
    for player in players {
        w.string(player);
    }
}
impl Codec<ScoreboardTeam> for ScoreboardTeam {
    fn read(r: &mut Reader) -> Result<ScoreboardTeam> {
        let team = r.string(16)?;
        match r.i8()? {
            0 => Ok(ScoreboardTeam::Create {
                team,
                display_name: r.string(32)?,
                prefix: r.string(16)?,
                suffix: r.string(16)?,
                friendly_fire: r.i8()?,
                name_tag_visibility: r.string(32)?,
                color: r.i8()?,
                players: read_players(r)?,
            }),
            1 => Ok(ScoreboardTeam::Remove { team }),
            2 => Ok(ScoreboardTeam::Update {
                team,
                display_name: r.string(32)?,
                prefix: r.string(16)?,
                suffix: r.string(16)?,
                friendly_fire: r.i8()?,
                name_tag_visibility: r.string(32)?,
                color: r.i8()?,
            }),
            3 => Ok(ScoreboardTeam::AddPlayers {
                team,
                players: read_players(r)?,
            }),
            4 => Ok(ScoreboardTeam::RemovePlayers {
                team,
                players: read_players(r)?,
            }),
            other => Err(crate::Error::new(format!("unknown team mode {other}"))),
        }
    }
    fn write(v: &ScoreboardTeam, w: &mut Writer) {
        match v {
            ScoreboardTeam::Create {
                team,
                display_name,
                prefix,
                suffix,
                friendly_fire,
                name_tag_visibility,
                color,
                players,
            } => {
                w.string(team);
                w.i8(0);
                w.string(display_name);
                w.string(prefix);
                w.string(suffix);
                w.i8(*friendly_fire);
                w.string(name_tag_visibility);
                w.i8(*color);
                write_players(players, w);
            }
            ScoreboardTeam::Remove { team } => {
                w.string(team);
                w.i8(1);
            }
            ScoreboardTeam::Update {
                team,
                display_name,
                prefix,
                suffix,
                friendly_fire,
                name_tag_visibility,
                color,
            } => {
                w.string(team);
                w.i8(2);
                w.string(display_name);
                w.string(prefix);
                w.string(suffix);
                w.i8(*friendly_fire);
                w.string(name_tag_visibility);
                w.i8(*color);
            }
            ScoreboardTeam::AddPlayers { team, players } => {
                w.string(team);
                w.i8(3);
                write_players(players, w);
            }
            ScoreboardTeam::RemovePlayers { team, players } => {
                w.string(team);
                w.i8(4);
                write_players(players, w);
            }
        }
    }
}

/// `PacketPlayOutWorldBorder`: six actions with disjoint field sets.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldBorder {
    SetSize {
        radius: f64,
    },
    LerpSize {
        old_radius: f64,
        new_radius: f64,
        speed: i64,
    },
    SetCenter {
        x: f64,
        z: f64,
    },
    Initialize {
        x: f64,
        z: f64,
        old_radius: f64,
        new_radius: f64,
        speed: i64,
        portal_boundary: i32,
        warning_time: i32,
        warning_blocks: i32,
    },
    SetWarningTime {
        warning_time: i32,
    },
    SetWarningBlocks {
        warning_blocks: i32,
    },
}
impl Codec<WorldBorder> for WorldBorder {
    fn read(r: &mut Reader) -> Result<WorldBorder> {
        match r.varint()? {
            0 => Ok(WorldBorder::SetSize { radius: r.f64()? }),
            1 => Ok(WorldBorder::LerpSize {
                old_radius: r.f64()?,
                new_radius: r.f64()?,
                speed: r.varlong()?,
            }),
            2 => Ok(WorldBorder::SetCenter {
                x: r.f64()?,
                z: r.f64()?,
            }),
            3 => Ok(WorldBorder::Initialize {
                x: r.f64()?,
                z: r.f64()?,
                old_radius: r.f64()?,
                new_radius: r.f64()?,
                speed: r.varlong()?,
                portal_boundary: r.varint()?,
                warning_time: r.varint()?,
                warning_blocks: r.varint()?,
            }),
            4 => Ok(WorldBorder::SetWarningTime {
                warning_time: r.varint()?,
            }),
            5 => Ok(WorldBorder::SetWarningBlocks {
                warning_blocks: r.varint()?,
            }),
            other => Err(crate::Error::new(format!(
                "unknown world border action {other}"
            ))),
        }
    }
    fn write(v: &WorldBorder, w: &mut Writer) {
        match v {
            WorldBorder::SetSize { radius } => {
                w.varint(0);
                w.f64(*radius);
            }
            WorldBorder::LerpSize {
                old_radius,
                new_radius,
                speed,
            } => {
                w.varint(1);
                w.f64(*old_radius);
                w.f64(*new_radius);
                w.varlong(*speed);
            }
            WorldBorder::SetCenter { x, z } => {
                w.varint(2);
                w.f64(*x);
                w.f64(*z);
            }
            WorldBorder::Initialize {
                x,
                z,
                old_radius,
                new_radius,
                speed,
                portal_boundary,
                warning_time,
                warning_blocks,
            } => {
                w.varint(3);
                w.f64(*x);
                w.f64(*z);
                w.f64(*old_radius);
                w.f64(*new_radius);
                w.varlong(*speed);
                w.varint(*portal_boundary);
                w.varint(*warning_time);
                w.varint(*warning_blocks);
            }
            WorldBorder::SetWarningTime { warning_time } => {
                w.varint(4);
                w.varint(*warning_time);
            }
            WorldBorder::SetWarningBlocks { warning_blocks } => {
                w.varint(5);
                w.varint(*warning_blocks);
            }
        }
    }
}

/// `PacketPlayInUseEntity`: the coordinates exist only for action 2.
#[derive(Clone, Debug, PartialEq)]
pub enum UseEntity {
    Interact { target: i32 },
    Attack { target: i32 },
    InteractAt { target: i32, x: f32, y: f32, z: f32 },
}
impl Codec<UseEntity> for UseEntity {
    fn read(r: &mut Reader) -> Result<UseEntity> {
        let target = r.varint()?;
        match r.varint()? {
            0 => Ok(UseEntity::Interact { target }),
            1 => Ok(UseEntity::Attack { target }),
            2 => Ok(UseEntity::InteractAt {
                target,
                x: r.f32()?,
                y: r.f32()?,
                z: r.f32()?,
            }),
            other => Err(crate::Error::new(format!(
                "unknown use-entity action {other}"
            ))),
        }
    }
    fn write(v: &UseEntity, w: &mut Writer) {
        match v {
            UseEntity::Interact { target } => {
                w.varint(*target);
                w.varint(0);
            }
            UseEntity::Attack { target } => {
                w.varint(*target);
                w.varint(1);
            }
            UseEntity::InteractAt { target, x, y, z } => {
                w.varint(*target);
                w.varint(2);
                w.f32(*x);
                w.f32(*y);
                w.f32(*z);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round<T: Clone + PartialEq + std::fmt::Debug + Codec<T>>(v: T) -> Vec<u8> {
        let mut w = Writer::new();
        <T as Codec<T>>::write(&v, &mut w);
        let mut r = Reader::new(&w.data);
        assert_eq!(<T as Codec<T>>::read(&mut r).unwrap(), v);
        assert_eq!(r.remaining(), 0);
        w.data
    }

    #[test]
    fn particle_data_length_depends_on_the_id() {
        assert_eq!(particle_data_len(0), 0);
        assert_eq!(particle_data_len(36), 2);
        assert_eq!(particle_data_len(37), 1);
        assert_eq!(particle_data_len(38), 1);
        let p = Particles {
            particle_id: 36,
            long_distance: true,
            position: [1.0, 2.0, 3.0],
            offset: [0.0; 3],
            speed: 0.5,
            count: 10,
            data: vec![1, 2],
        };
        let bytes = round(p.clone());
        assert_eq!(bytes[4], 1);
        assert_eq!(&bytes[bytes.len() - 2..], &[1, 2]);
        // A wrong data vector still writes exactly the id's fields.
        let mut bad = p.clone();
        bad.data = vec![7; 5];
        let mut w = Writer::new();
        <Particles as Codec<Particles>>::write(&bad, &mut w);
        assert_eq!(&w.data[w.data.len() - 2..], &[7, 7]);
    }

    #[test]
    fn spawn_object_velocity_follows_object_data() {
        let still = SpawnObject {
            entity_id: 1,
            kind: 2,
            position: [0, 64, 0],
            pitch: 0,
            yaw: 0,
            object_data: 0,
            velocity: Some([1, 2, 3]),
        };
        let mut w = Writer::new();
        <SpawnObject as Codec<SpawnObject>>::write(&still, &mut w);
        assert_eq!(w.data.len(), 1 + 1 + 12 + 2 + 4);
        let mut r = Reader::new(&w.data);
        assert_eq!(
            <SpawnObject as Codec<SpawnObject>>::read(&mut r)
                .unwrap()
                .velocity,
            None
        );
        let moving = SpawnObject {
            object_data: 1,
            velocity: Some([1, 2, 3]),
            ..SpawnObject {
                entity_id: 1,
                kind: 2,
                position: [0, 64, 0],
                pitch: 0,
                yaw: 0,
                object_data: 0,
                velocity: None,
            }
        };
        assert_eq!(round(moving).len(), 1 + 1 + 12 + 2 + 4 + 6);
    }

    #[test]
    fn explosion_and_attributes_round_trip() {
        let explosion = Explosion {
            position: [1.0, 2.0, 3.0],
            radius: 4.0,
            affected_blocks: vec![[1, -1, 2], [-3, 0, 5]],
            player_motion: [0.1, 0.2, 0.3],
        };
        let bytes = round(explosion);
        assert_eq!(&bytes[20..22], &[1, 0xFF]);

        let attributes = EntityProperties {
            entity_id: 3,
            properties: vec![Property {
                key: "generic.maxHealth".into(),
                value: 20.0,
                modifiers: vec![Modifier {
                    uuid: 1,
                    amount: 2.0,
                    operation: 0,
                }],
            }],
        };
        round(attributes);
    }

    #[test]
    fn map_update_with_and_without_a_patch() {
        let empty = MapUpdate {
            item_damage: 0,
            scale: 1,
            icons: Vec::new(),
            patch: None,
        };
        assert_eq!(round(empty.clone()), [0x00, 0x01, 0x00, 0x00]);
        let full = MapUpdate {
            item_damage: 5,
            scale: 2,
            icons: vec![MapIcon {
                direction_and_type: 0x13,
                x: 1,
                z: -1,
            }],
            patch: Some(MapPatch {
                columns: 2,
                rows: 1,
                x: -1,
                y: 0,
                data: vec![1, 2],
            }),
        };
        round(full);
        let mut wrong = empty;
        wrong.patch = Some(MapPatch {
            columns: 0,
            rows: 0,
            x: 0,
            y: 0,
            data: vec![9],
        });
        let mut w = Writer::new();
        <MapUpdate as Codec<MapUpdate>>::write(&wrong, &mut w);
        assert_eq!(
            w.data,
            [0x00, 0x01, 0x00, 0x00],
            "a zero-column patch is not written"
        );
    }

    #[test]
    fn scoreboard_packets_round_trip() {
        for objective in [
            ScoreboardObjective::Create {
                name: "obj".into(),
                display_name: "Display".into(),
                kind: "integer".into(),
            },
            ScoreboardObjective::Remove { name: "obj".into() },
            ScoreboardObjective::Update {
                name: "obj".into(),
                display_name: "Other".into(),
                kind: "hearts".into(),
            },
        ] {
            round(objective);
        }
        for score in [
            ScoreboardScore {
                entity_name: "player".into(),
                action: 0,
                objective_name: "obj".into(),
                value: Some(10),
            },
            ScoreboardScore {
                entity_name: "player".into(),
                action: 1,
                objective_name: "obj".into(),
                value: None,
            },
        ] {
            round(score);
        }
        round(DisplayObjective {
            position: 1,
            name: "obj".into(),
        });
        for team in [
            ScoreboardTeam::Create {
                team: "team".into(),
                display_name: "Team".into(),
                prefix: "[".into(),
                suffix: "]".into(),
                friendly_fire: 3,
                name_tag_visibility: "always".into(),
                color: 2,
                players: vec!["a".into(), "b".into()],
            },
            ScoreboardTeam::Remove {
                team: "team".into(),
            },
            ScoreboardTeam::Update {
                team: "team".into(),
                display_name: "T".into(),
                prefix: "".into(),
                suffix: "".into(),
                friendly_fire: 0,
                name_tag_visibility: "never".into(),
                color: -1,
            },
            ScoreboardTeam::AddPlayers {
                team: "team".into(),
                players: vec!["c".into()],
            },
            ScoreboardTeam::RemovePlayers {
                team: "team".into(),
                players: vec!["d".into()],
            },
        ] {
            round(team);
        }
    }

    #[test]
    fn scoreboard_score_remove_has_no_value() {
        let mut w = Writer::new();
        <ScoreboardScore as Codec<ScoreboardScore>>::write(
            &ScoreboardScore {
                entity_name: "p".into(),
                action: 1,
                objective_name: "o".into(),
                value: Some(99),
            },
            &mut w,
        );
        assert_eq!(w.data, [0x01, b'p', 0x01, 0x01, b'o']);
    }

    #[test]
    fn world_border_actions_round_trip() {
        for border in [
            WorldBorder::SetSize { radius: 100.0 },
            WorldBorder::LerpSize {
                old_radius: 100.0,
                new_radius: 200.0,
                speed: 1000,
            },
            WorldBorder::SetCenter { x: 1.0, z: -2.0 },
            WorldBorder::Initialize {
                x: 1.0,
                z: 2.0,
                old_radius: 3.0,
                new_radius: 4.0,
                speed: 5,
                portal_boundary: 6,
                warning_time: 7,
                warning_blocks: 8,
            },
            WorldBorder::SetWarningTime { warning_time: 15 },
            WorldBorder::SetWarningBlocks { warning_blocks: 5 },
        ] {
            round(border);
        }
        let mut w = Writer::new();
        <WorldBorder as Codec<WorldBorder>>::write(&WorldBorder::SetSize { radius: 1.0 }, &mut w);
        assert_eq!(w.data[0], 0);
        assert_eq!(w.data.len(), 1 + 8);
    }

    #[test]
    fn use_entity_coordinates_only_for_action_two() {
        assert_eq!(round(UseEntity::Interact { target: 3 }), [0x03, 0x00]);
        assert_eq!(round(UseEntity::Attack { target: 3 }), [0x03, 0x01]);
        let at = UseEntity::InteractAt {
            target: 3,
            x: 0.5,
            y: 1.0,
            z: -0.5,
        };
        let bytes = round(at);
        assert_eq!(bytes.len(), 2 + 12);
    }

    #[test]
    fn statistics_round_trip() {
        round(Statistics(vec![
            ("stat.jump".into(), 10),
            ("stat.walkOneCm".into(), 1234),
        ]));
        round(Statistics::default());
    }

    #[test]
    fn unknown_discriminators_are_rejected() {
        assert!(
            <ScoreboardObjective as Codec<ScoreboardObjective>>::read(&mut Reader::new(&[
                0x01, b'o', 0x07
            ]))
            .is_err()
        );
        assert!(
            <ScoreboardTeam as Codec<ScoreboardTeam>>::read(&mut Reader::new(&[0x01, b't', 0x09]))
                .is_err()
        );
        assert!(<WorldBorder as Codec<WorldBorder>>::read(&mut Reader::new(&[0x09])).is_err());
        assert!(<UseEntity as Codec<UseEntity>>::read(&mut Reader::new(&[0x01, 0x09])).is_err());
    }
}
