//! Packet ids for Minecraft Java Edition 1.8.9 (protocol 47).
//!
//! Only the states modeled in this module are listed; an id that is absent
//! decodes as `Unknown` and keeps its payload.

pub mod handshake {
    pub mod serverbound {
        pub const INTENTION: i32 = 0x00;
    }
}

pub mod status {
    pub mod clientbound {
        pub const STATUS_RESPONSE: i32 = 0x00;
        pub const PONG_RESPONSE: i32 = 0x01;
    }
    pub mod serverbound {
        pub const STATUS_REQUEST: i32 = 0x00;
        pub const PING_REQUEST: i32 = 0x01;
    }
}

pub mod login {
    pub mod clientbound {
        pub const LOGIN_DISCONNECT: i32 = 0x00;
        pub const ENCRYPTION_REQUEST: i32 = 0x01;
        pub const LOGIN_SUCCESS: i32 = 0x02;
        pub const SET_COMPRESSION: i32 = 0x03;
    }
    pub mod serverbound {
        pub const LOGIN_START: i32 = 0x00;
        pub const ENCRYPTION_RESPONSE: i32 = 0x01;
    }
}

pub mod play {
    pub mod clientbound {
        pub const KEEP_ALIVE: i32 = 0x00;
        pub const JOIN_GAME: i32 = 0x01;
        pub const CHAT: i32 = 0x02;
        pub const TIME_UPDATE: i32 = 0x03;
        pub const ENTITY_EQUIPMENT: i32 = 0x04;
        pub const SPAWN_POSITION: i32 = 0x05;
        pub const UPDATE_HEALTH: i32 = 0x06;
        pub const RESPAWN: i32 = 0x07;
        pub const PLAYER_POSITION_AND_LOOK: i32 = 0x08;
        pub const HELD_ITEM_CHANGE: i32 = 0x09;
        pub const USE_BED: i32 = 0x0A;
        pub const ENTITY_ANIMATION: i32 = 0x0B;
        pub const SPAWN_PLAYER: i32 = 0x0C;
        pub const COLLECT_ITEM: i32 = 0x0D;
        pub const SPAWN_OBJECT: i32 = 0x0E;
        pub const SPAWN_MOB: i32 = 0x0F;
        pub const SPAWN_PAINTING: i32 = 0x10;
        pub const SPAWN_EXPERIENCE_ORB: i32 = 0x11;
        pub const ENTITY_VELOCITY: i32 = 0x12;
        pub const DESTROY_ENTITIES: i32 = 0x13;
        pub const ENTITY: i32 = 0x14;
        pub const ENTITY_RELATIVE_MOVE: i32 = 0x15;
        pub const ENTITY_LOOK: i32 = 0x16;
        pub const ENTITY_LOOK_AND_RELATIVE_MOVE: i32 = 0x17;
        pub const ENTITY_TELEPORT: i32 = 0x18;
        pub const ENTITY_HEAD_LOOK: i32 = 0x19;
        pub const ENTITY_STATUS: i32 = 0x1A;
        pub const ATTACH_ENTITY: i32 = 0x1B;
        pub const ENTITY_METADATA: i32 = 0x1C;
        pub const ENTITY_EFFECT: i32 = 0x1D;
        pub const REMOVE_ENTITY_EFFECT: i32 = 0x1E;
        pub const SET_EXPERIENCE: i32 = 0x1F;
        pub const ENTITY_PROPERTIES: i32 = 0x20;
        pub const MAP_CHUNK: i32 = 0x21;
        pub const MULTI_BLOCK_CHANGE: i32 = 0x22;
        pub const BLOCK_CHANGE: i32 = 0x23;
        pub const BLOCK_ACTION: i32 = 0x24;
        pub const BLOCK_BREAK_ANIMATION: i32 = 0x25;
        pub const MAP_CHUNK_BULK: i32 = 0x26;
        pub const EXPLOSION: i32 = 0x27;
        pub const WORLD_EVENT: i32 = 0x28;
        pub const SOUND_EFFECT: i32 = 0x29;
        pub const WORLD_PARTICLES: i32 = 0x2A;
        pub const GAME_STATE_CHANGE: i32 = 0x2B;
        pub const SPAWN_WEATHER: i32 = 0x2C;
        pub const OPEN_WINDOW: i32 = 0x2D;
        pub const CLOSE_WINDOW: i32 = 0x2E;
        pub const SET_SLOT: i32 = 0x2F;
        pub const WINDOW_ITEMS: i32 = 0x30;
        pub const WINDOW_PROPERTY: i32 = 0x31;
        pub const CONFIRM_TRANSACTION: i32 = 0x32;
        pub const UPDATE_SIGN: i32 = 0x33;
        pub const MAP: i32 = 0x34;
        pub const TILE_ENTITY_DATA: i32 = 0x35;
        pub const OPEN_SIGN_EDITOR: i32 = 0x36;
        pub const STATISTICS: i32 = 0x37;
        pub const PLAYER_INFO: i32 = 0x38;
        pub const ABILITIES: i32 = 0x39;
        pub const TAB_COMPLETE: i32 = 0x3A;
        pub const SCOREBOARD_OBJECTIVE: i32 = 0x3B;
        pub const SCOREBOARD_SCORE: i32 = 0x3C;
        pub const DISPLAY_SCOREBOARD: i32 = 0x3D;
        pub const SCOREBOARD_TEAM: i32 = 0x3E;
        pub const CUSTOM_PAYLOAD: i32 = 0x3F;
        pub const DISCONNECT: i32 = 0x40;
        pub const DIFFICULTY: i32 = 0x41;
        pub const COMBAT_EVENT: i32 = 0x42;
        pub const CAMERA: i32 = 0x43;
        pub const WORLD_BORDER: i32 = 0x44;
        pub const TITLE: i32 = 0x45;
        pub const SET_COMPRESSION: i32 = 0x46;
        pub const PLAYER_LIST_HEADER: i32 = 0x47;
        pub const RESOURCE_PACK_SEND: i32 = 0x48;
        pub const UPDATE_ENTITY_NBT: i32 = 0x49;
    }
    pub mod serverbound {
        pub const KEEP_ALIVE: i32 = 0x00;
        pub const CHAT: i32 = 0x01;
        pub const USE_ENTITY: i32 = 0x02;
        pub const FLYING: i32 = 0x03;
        pub const PLAYER_POSITION: i32 = 0x04;
        pub const PLAYER_LOOK: i32 = 0x05;
        pub const PLAYER_POSITION_AND_LOOK: i32 = 0x06;
        pub const PLAYER_DIGGING: i32 = 0x07;
        pub const BLOCK_PLACEMENT: i32 = 0x08;
        pub const HELD_ITEM_CHANGE: i32 = 0x09;
        pub const ANIMATION: i32 = 0x0A;
        pub const ENTITY_ACTION: i32 = 0x0B;
        pub const STEER_VEHICLE: i32 = 0x0C;
        pub const CLOSE_WINDOW: i32 = 0x0D;
        pub const WINDOW_CLICK: i32 = 0x0E;
        pub const TRANSACTION: i32 = 0x0F;
        pub const SET_CREATIVE_SLOT: i32 = 0x10;
        pub const ENCHANT_ITEM: i32 = 0x11;
        pub const UPDATE_SIGN: i32 = 0x12;
        pub const ABILITIES: i32 = 0x13;
        pub const TAB_COMPLETE: i32 = 0x14;
        pub const CLIENT_SETTINGS: i32 = 0x15;
        pub const CLIENT_COMMAND: i32 = 0x16;
        pub const PLUGIN_MESSAGE: i32 = 0x17;
        pub const SPECTATE: i32 = 0x18;
        pub const RESOURCE_PACK_STATUS: i32 = 0x19;
    }
}
