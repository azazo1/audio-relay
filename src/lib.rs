mod codec;
mod config;
mod error;
mod fec;
mod platform;
mod protocol;
mod receiver;
mod sender;

pub mod capture;
pub mod playback;

pub use codec::{OpusDecoder, OpusEncoder, OpusMultistreamConfig};
pub use config::{
    AudioLayout, CaptureConfig, CodecConfig, DEFAULT_INITIAL_DROP_MS, DEFAULT_PACKET_DURATION_MS,
    PlaybackConfig, ReceiverConfig, SAMPLE_RATE, SenderConfig, StreamParams,
};
pub use error::{Error, Result};
pub use protocol::{
    AUDIO_FEC_HEADER_LEN, AudioFecHeader, ParsedPacket, RTP_HEADER_LEN, RTP_PAYLOAD_TYPE_AUDIO,
    RTP_PAYLOAD_TYPE_FEC, RTPA_DATA_SHARDS, RTPA_FEC_SHARDS, RTPA_TOTAL_SHARDS, RtpHeader,
};
pub use receiver::{
    AudioDepacketizer, AudioReceiver, QueuedAudioFrame, RtpAudioQueue, RtpAudioStats,
};
pub use sender::{AudioPacketizer, AudioSender, OutboundDatagram};

#[cfg(test)]
mod tests {
    use super::{AudioDepacketizer, AudioPacketizer, QueuedAudioFrame, fec};

    #[test]
    fn audio_fec_recovers_single_missing_shard() {
        let shard0 = vec![1u8; 32];
        let shard1 = vec![2u8; 32];
        let shard2 = vec![3u8; 32];
        let shard3 = vec![4u8; 32];
        let mut parity0 = vec![0u8; 32];
        let mut parity1 = vec![0u8; 32];

        fec::encode_audio_block(
            [&shard0, &shard1, &shard2, &shard3],
            [&mut parity0, &mut parity1],
        )
        .unwrap();

        let mut data = [Some(shard0), None, Some(shard2), Some(shard3)];
        let parity = [Some(parity0), Some(parity1)];
        let recovered = fec::recover_audio_block(&mut data, &parity).unwrap();

        assert_eq!(recovered, 1);
        assert_eq!(data[1].as_deref(), Some(&[2u8; 32][..]));
    }

    #[test]
    fn packetizer_and_depacketizer_round_trip_second_block_with_fec() {
        let mut packetizer = AudioPacketizer::new(5, 0, true);
        let mut depacketizer = AudioDepacketizer::new(5, 0);

        let mut datagrams = Vec::new();
        for index in 0..8u8 {
            let payload = vec![index + 10; 24];
            datagrams.extend(packetizer.push_encoded_frame(&payload).unwrap());
        }

        let mut ready = Vec::new();
        for datagram in datagrams {
            let is_dropped_second_block_data =
                datagram.packet_type == 97 && datagram.sequence_number == 5;
            if is_dropped_second_block_data {
                continue;
            }
            ready.extend(depacketizer.push_datagram(&datagram.bytes).unwrap());
        }

        let decoded_payloads: Vec<Vec<u8>> = ready
            .into_iter()
            .filter_map(|frame| match frame {
                QueuedAudioFrame::Encoded(packet) => Some(packet),
                QueuedAudioFrame::Missing => None,
            })
            .collect();

        assert_eq!(decoded_payloads.len(), 4);
        assert_eq!(decoded_payloads[0], vec![14u8; 24]);
        assert_eq!(decoded_payloads[1], vec![15u8; 24]);
        assert_eq!(decoded_payloads[2], vec![16u8; 24]);
        assert_eq!(decoded_payloads[3], vec![17u8; 24]);
    }
}
