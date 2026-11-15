//! Status state: the 1.8.9 server-list ping.
//!
//! The status state is unchanged between 1.8.9 and 26.3: request, JSON
//! response, ping, pong.
use crate::{
    buf::{Long, Str},
    packet, packet_enum,
    packets::Empty,
};

packet!(StatusResponse { json: String => Str });
packet!(Ping { time: i64 => Long });

packet_enum!(Clientbound, crate::v1_8_9::ids::status::clientbound {
    StatusResponse(StatusResponse) = STATUS_RESPONSE,
    PongResponse(Ping) = PONG_RESPONSE,
});
packet_enum!(Serverbound, crate::v1_8_9::ids::status::serverbound {
    StatusRequest(Empty) = STATUS_REQUEST,
    PingRequest(Ping) = PING_REQUEST,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::round_trip;

    #[test]
    fn status_request_is_one_empty_packet() {
        assert_eq!(Serverbound::StatusRequest(Empty).encode(), [0x00]);
    }

    #[test]
    fn status_round_trips() {
        round_trip(
            Clientbound::StatusResponse(StatusResponse {
                json: "{\"version\":{\"name\":\"1.8.9\",\"protocol\":47}}".into(),
            }),
            Clientbound::encode,
            Clientbound::decode,
        );
        round_trip(
            Clientbound::PongResponse(Ping { time: 123 }),
            Clientbound::encode,
            Clientbound::decode,
        );
        round_trip(
            Serverbound::PingRequest(Ping { time: -5 }),
            Serverbound::encode,
            Serverbound::decode,
        );
    }
}
