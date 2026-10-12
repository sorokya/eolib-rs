//! Round-trip tests for every packet captured in `eo-captured-packets`.
//!
//! Each JSON file in the [`eo-captured-packets`] submodule (the same data used by
//! `../eolib-c` to generate its `packet_tests.c`) contains the raw packet body as
//! produced by the original v28 client/server. These tests feed those bytes through
//! the matching generated packet type and assert that serializing the deserialized
//! packet reproduces the original bytes exactly, covering both directions at once.
//!
//! [`eo-captured-packets`]: https://github.com/ethanmoffat/eo-captured-packets

use std::{fs, path::Path};

use bytes::Bytes;
use eolib::data::{EoReader, EoSerialize, EoWriter};

/// Signature of a round-trip function for a single packet type.
type Roundtrip = fn(&[u8]) -> Result<Vec<u8>, String>;

/// Table mapping a captured packet's direction, family, and action to the generated
/// packet type that models it. Kept in sync with the captured packets by
/// `tools/generate_packet_roundtrip.py`.
#[rustfmt::skip]
const PACKETS: &[(&str, &str, &str, Roundtrip)] = &[
    ("client", "Account", "Agree", roundtrip::<eolib::protocol::net::client::AccountAgreeClientPacket>),
    ("client", "Account", "Create", roundtrip::<eolib::protocol::net::client::AccountCreateClientPacket>),
    ("client", "Account", "Request", roundtrip::<eolib::protocol::net::client::AccountRequestClientPacket>),
    ("client", "Attack", "Use", roundtrip::<eolib::protocol::net::client::AttackUseClientPacket>),
    ("client", "Bank", "Add", roundtrip::<eolib::protocol::net::client::BankAddClientPacket>),
    ("client", "Bank", "Open", roundtrip::<eolib::protocol::net::client::BankOpenClientPacket>),
    ("client", "Bank", "Take", roundtrip::<eolib::protocol::net::client::BankTakeClientPacket>),
    ("client", "Board", "Create", roundtrip::<eolib::protocol::net::client::BoardCreateClientPacket>),
    ("client", "Board", "Open", roundtrip::<eolib::protocol::net::client::BoardOpenClientPacket>),
    ("client", "Board", "Remove", roundtrip::<eolib::protocol::net::client::BoardRemoveClientPacket>),
    ("client", "Board", "Take", roundtrip::<eolib::protocol::net::client::BoardTakeClientPacket>),
    ("client", "Book", "Request", roundtrip::<eolib::protocol::net::client::BookRequestClientPacket>),
    ("client", "Chair", "Request", roundtrip::<eolib::protocol::net::client::ChairRequestClientPacket>),
    ("client", "Character", "Create", roundtrip::<eolib::protocol::net::client::CharacterCreateClientPacket>),
    ("client", "Character", "Remove", roundtrip::<eolib::protocol::net::client::CharacterRemoveClientPacket>),
    ("client", "Character", "Request", roundtrip::<eolib::protocol::net::client::CharacterRequestClientPacket>),
    ("client", "Character", "Take", roundtrip::<eolib::protocol::net::client::CharacterTakeClientPacket>),
    ("client", "Chest", "Add", roundtrip::<eolib::protocol::net::client::ChestAddClientPacket>),
    ("client", "Chest", "Open", roundtrip::<eolib::protocol::net::client::ChestOpenClientPacket>),
    ("client", "Chest", "Take", roundtrip::<eolib::protocol::net::client::ChestTakeClientPacket>),
    ("client", "Citizen", "Accept", roundtrip::<eolib::protocol::net::client::CitizenAcceptClientPacket>),
    ("client", "Citizen", "Open", roundtrip::<eolib::protocol::net::client::CitizenOpenClientPacket>),
    ("client", "Citizen", "Remove", roundtrip::<eolib::protocol::net::client::CitizenRemoveClientPacket>),
    ("client", "Citizen", "Reply", roundtrip::<eolib::protocol::net::client::CitizenReplyClientPacket>),
    ("client", "Citizen", "Request", roundtrip::<eolib::protocol::net::client::CitizenRequestClientPacket>),
    ("client", "Connection", "Accept", roundtrip::<eolib::protocol::net::client::ConnectionAcceptClientPacket>),
    ("client", "Connection", "Ping", roundtrip::<eolib::protocol::net::client::ConnectionPingClientPacket>),
    ("client", "Door", "Open", roundtrip::<eolib::protocol::net::client::DoorOpenClientPacket>),
    ("client", "Emote", "Report", roundtrip::<eolib::protocol::net::client::EmoteReportClientPacket>),
    ("client", "Face", "Player", roundtrip::<eolib::protocol::net::client::FacePlayerClientPacket>),
    ("client", "Global", "Close", roundtrip::<eolib::protocol::net::client::GlobalCloseClientPacket>),
    ("client", "Global", "Open", roundtrip::<eolib::protocol::net::client::GlobalOpenClientPacket>),
    ("client", "Global", "Player", roundtrip::<eolib::protocol::net::client::GlobalPlayerClientPacket>),
    ("client", "Global", "Remove", roundtrip::<eolib::protocol::net::client::GlobalRemoveClientPacket>),
    ("client", "Init", "Init", roundtrip::<eolib::protocol::net::client::InitInitClientPacket>),
    ("client", "Item", "Get", roundtrip::<eolib::protocol::net::client::ItemGetClientPacket>),
    ("client", "Item", "Use", roundtrip::<eolib::protocol::net::client::ItemUseClientPacket>),
    ("client", "Jukebox", "Msg", roundtrip::<eolib::protocol::net::client::JukeboxMsgClientPacket>),
    ("client", "Jukebox", "Open", roundtrip::<eolib::protocol::net::client::JukeboxOpenClientPacket>),
    ("client", "Locker", "Add", roundtrip::<eolib::protocol::net::client::LockerAddClientPacket>),
    ("client", "Locker", "Buy", roundtrip::<eolib::protocol::net::client::LockerBuyClientPacket>),
    ("client", "Locker", "Open", roundtrip::<eolib::protocol::net::client::LockerOpenClientPacket>),
    ("client", "Locker", "Take", roundtrip::<eolib::protocol::net::client::LockerTakeClientPacket>),
    ("client", "Login", "Request", roundtrip::<eolib::protocol::net::client::LoginRequestClientPacket>),
    ("client", "Marriage", "Open", roundtrip::<eolib::protocol::net::client::MarriageOpenClientPacket>),
    ("client", "NpcRange", "Request", roundtrip::<eolib::protocol::net::client::NpcRangeRequestClientPacket>),
    ("client", "Paperdoll", "Add", roundtrip::<eolib::protocol::net::client::PaperdollAddClientPacket>),
    ("client", "Paperdoll", "Remove", roundtrip::<eolib::protocol::net::client::PaperdollRemoveClientPacket>),
    ("client", "Paperdoll", "Request", roundtrip::<eolib::protocol::net::client::PaperdollRequestClientPacket>),
    ("client", "Party", "Remove", roundtrip::<eolib::protocol::net::client::PartyRemoveClientPacket>),
    ("client", "Party", "Request", roundtrip::<eolib::protocol::net::client::PartyRequestClientPacket>),
    ("client", "Party", "Take", roundtrip::<eolib::protocol::net::client::PartyTakeClientPacket>),
    ("client", "Players", "Accept", roundtrip::<eolib::protocol::net::client::PlayersAcceptClientPacket>),
    ("client", "Players", "List", roundtrip::<eolib::protocol::net::client::PlayersListClientPacket>),
    ("client", "Players", "Request", roundtrip::<eolib::protocol::net::client::PlayersRequestClientPacket>),
    ("client", "Quest", "Accept", roundtrip::<eolib::protocol::net::client::QuestAcceptClientPacket>),
    ("client", "Quest", "List", roundtrip::<eolib::protocol::net::client::QuestListClientPacket>),
    ("client", "Quest", "Use", roundtrip::<eolib::protocol::net::client::QuestUseClientPacket>),
    ("client", "Range", "Request", roundtrip::<eolib::protocol::net::client::RangeRequestClientPacket>),
    ("client", "Shop", "Open", roundtrip::<eolib::protocol::net::client::ShopOpenClientPacket>),
    ("client", "Sit", "Request", roundtrip::<eolib::protocol::net::client::SitRequestClientPacket>),
    ("client", "Spell", "Request", roundtrip::<eolib::protocol::net::client::SpellRequestClientPacket>),
    ("client", "Spell", "TargetOther", roundtrip::<eolib::protocol::net::client::SpellTargetOtherClientPacket>),
    ("client", "StatSkill", "Open", roundtrip::<eolib::protocol::net::client::StatSkillOpenClientPacket>),
    ("client", "Talk", "Msg", roundtrip::<eolib::protocol::net::client::TalkMsgClientPacket>),
    ("client", "Talk", "Open", roundtrip::<eolib::protocol::net::client::TalkOpenClientPacket>),
    ("client", "Talk", "Report", roundtrip::<eolib::protocol::net::client::TalkReportClientPacket>),
    ("client", "Talk", "Tell", roundtrip::<eolib::protocol::net::client::TalkTellClientPacket>),
    ("client", "Trade", "Accept", roundtrip::<eolib::protocol::net::client::TradeAcceptClientPacket>),
    ("client", "Trade", "Add", roundtrip::<eolib::protocol::net::client::TradeAddClientPacket>),
    ("client", "Trade", "Agree", roundtrip::<eolib::protocol::net::client::TradeAgreeClientPacket>),
    ("client", "Trade", "Remove", roundtrip::<eolib::protocol::net::client::TradeRemoveClientPacket>),
    ("client", "Trade", "Request", roundtrip::<eolib::protocol::net::client::TradeRequestClientPacket>),
    ("client", "Walk", "Player", roundtrip::<eolib::protocol::net::client::WalkPlayerClientPacket>),
    ("client", "Warp", "Accept", roundtrip::<eolib::protocol::net::client::WarpAcceptClientPacket>),
    ("client", "Warp", "Take", roundtrip::<eolib::protocol::net::client::WarpTakeClientPacket>),
    ("client", "Welcome", "Agree", roundtrip::<eolib::protocol::net::client::WelcomeAgreeClientPacket>),
    ("client", "Welcome", "Msg", roundtrip::<eolib::protocol::net::client::WelcomeMsgClientPacket>),
    ("client", "Welcome", "Request", roundtrip::<eolib::protocol::net::client::WelcomeRequestClientPacket>),
    ("server", "Account", "Reply", roundtrip::<eolib::protocol::net::server::AccountReplyServerPacket>),
    ("server", "AdminInteract", "Agree", roundtrip::<eolib::protocol::net::server::AdminInteractAgreeServerPacket>),
    ("server", "AdminInteract", "List", roundtrip::<eolib::protocol::net::server::AdminInteractListServerPacket>),
    ("server", "AdminInteract", "Remove", roundtrip::<eolib::protocol::net::server::AdminInteractRemoveServerPacket>),
    ("server", "AdminInteract", "Tell", roundtrip::<eolib::protocol::net::server::AdminInteractTellServerPacket>),
    ("server", "Attack", "Player", roundtrip::<eolib::protocol::net::server::AttackPlayerServerPacket>),
    ("server", "Avatar", "Remove", roundtrip::<eolib::protocol::net::server::AvatarRemoveServerPacket>),
    ("server", "Bank", "Open", roundtrip::<eolib::protocol::net::server::BankOpenServerPacket>),
    ("server", "Bank", "Reply", roundtrip::<eolib::protocol::net::server::BankReplyServerPacket>),
    ("server", "Board", "Open", roundtrip::<eolib::protocol::net::server::BoardOpenServerPacket>),
    ("server", "Board", "Player", roundtrip::<eolib::protocol::net::server::BoardPlayerServerPacket>),
    ("server", "Book", "Reply", roundtrip::<eolib::protocol::net::server::BookReplyServerPacket>),
    ("server", "Cast", "Reply", roundtrip::<eolib::protocol::net::server::CastReplyServerPacket>),
    ("server", "Cast", "Spec", roundtrip::<eolib::protocol::net::server::CastSpecServerPacket>),
    ("server", "Chair", "Close", roundtrip::<eolib::protocol::net::server::ChairCloseServerPacket>),
    ("server", "Chair", "Reply", roundtrip::<eolib::protocol::net::server::ChairReplyServerPacket>),
    ("server", "Character", "Player", roundtrip::<eolib::protocol::net::server::CharacterPlayerServerPacket>),
    ("server", "Character", "Reply", roundtrip::<eolib::protocol::net::server::CharacterReplyServerPacket>),
    ("server", "Chest", "Get", roundtrip::<eolib::protocol::net::server::ChestGetServerPacket>),
    ("server", "Chest", "Open", roundtrip::<eolib::protocol::net::server::ChestOpenServerPacket>),
    ("server", "Chest", "Reply", roundtrip::<eolib::protocol::net::server::ChestReplyServerPacket>),
    ("server", "Citizen", "Accept", roundtrip::<eolib::protocol::net::server::CitizenAcceptServerPacket>),
    ("server", "Citizen", "Open", roundtrip::<eolib::protocol::net::server::CitizenOpenServerPacket>),
    ("server", "Citizen", "Remove", roundtrip::<eolib::protocol::net::server::CitizenRemoveServerPacket>),
    ("server", "Citizen", "Reply", roundtrip::<eolib::protocol::net::server::CitizenReplyServerPacket>),
    ("server", "Citizen", "Request", roundtrip::<eolib::protocol::net::server::CitizenRequestServerPacket>),
    ("server", "Connection", "Player", roundtrip::<eolib::protocol::net::server::ConnectionPlayerServerPacket>),
    ("server", "Door", "Open", roundtrip::<eolib::protocol::net::server::DoorOpenServerPacket>),
    ("server", "Effect", "Use", roundtrip::<eolib::protocol::net::server::EffectUseServerPacket>),
    ("server", "Emote", "Player", roundtrip::<eolib::protocol::net::server::EmotePlayerServerPacket>),
    ("server", "Face", "Player", roundtrip::<eolib::protocol::net::server::FacePlayerServerPacket>),
    ("server", "Init", "Init", roundtrip::<eolib::protocol::net::server::InitInitServerPacket>),
    ("server", "Item", "Add", roundtrip::<eolib::protocol::net::server::ItemAddServerPacket>),
    ("server", "Item", "Get", roundtrip::<eolib::protocol::net::server::ItemGetServerPacket>),
    ("server", "Item", "Remove", roundtrip::<eolib::protocol::net::server::ItemRemoveServerPacket>),
    ("server", "Item", "Reply", roundtrip::<eolib::protocol::net::server::ItemReplyServerPacket>),
    ("server", "Item", "Spec", roundtrip::<eolib::protocol::net::server::ItemSpecServerPacket>),
    ("server", "Jukebox", "Agree", roundtrip::<eolib::protocol::net::server::JukeboxAgreeServerPacket>),
    ("server", "Jukebox", "Open", roundtrip::<eolib::protocol::net::server::JukeboxOpenServerPacket>),
    ("server", "Jukebox", "Player", roundtrip::<eolib::protocol::net::server::JukeboxPlayerServerPacket>),
    ("server", "Jukebox", "Use", roundtrip::<eolib::protocol::net::server::JukeboxUseServerPacket>),
    ("server", "Locker", "Buy", roundtrip::<eolib::protocol::net::server::LockerBuyServerPacket>),
    ("server", "Locker", "Get", roundtrip::<eolib::protocol::net::server::LockerGetServerPacket>),
    ("server", "Locker", "Open", roundtrip::<eolib::protocol::net::server::LockerOpenServerPacket>),
    ("server", "Locker", "Reply", roundtrip::<eolib::protocol::net::server::LockerReplyServerPacket>),
    ("server", "Login", "Reply", roundtrip::<eolib::protocol::net::server::LoginReplyServerPacket>),
    ("server", "Marriage", "Open", roundtrip::<eolib::protocol::net::server::MarriageOpenServerPacket>),
    ("server", "Message", "Open", roundtrip::<eolib::protocol::net::server::MessageOpenServerPacket>),
    ("server", "Npc", "Agree", roundtrip::<eolib::protocol::net::server::NpcAgreeServerPacket>),
    ("server", "Npc", "Player", roundtrip::<eolib::protocol::net::server::NpcPlayerServerPacket>),
    ("server", "Npc", "Reply", roundtrip::<eolib::protocol::net::server::NpcReplyServerPacket>),
    ("server", "Npc", "Spec", roundtrip::<eolib::protocol::net::server::NpcSpecServerPacket>),
    ("server", "Paperdoll", "Agree", roundtrip::<eolib::protocol::net::server::PaperdollAgreeServerPacket>),
    ("server", "Paperdoll", "Remove", roundtrip::<eolib::protocol::net::server::PaperdollRemoveServerPacket>),
    ("server", "Paperdoll", "Reply", roundtrip::<eolib::protocol::net::server::PaperdollReplyServerPacket>),
    ("server", "Party", "Agree", roundtrip::<eolib::protocol::net::server::PartyAgreeServerPacket>),
    ("server", "Party", "Create", roundtrip::<eolib::protocol::net::server::PartyCreateServerPacket>),
    ("server", "Party", "Remove", roundtrip::<eolib::protocol::net::server::PartyRemoveServerPacket>),
    ("server", "Party", "Request", roundtrip::<eolib::protocol::net::server::PartyRequestServerPacket>),
    ("server", "Party", "TargetGroup", roundtrip::<eolib::protocol::net::server::PartyTargetGroupServerPacket>),
    ("server", "Players", "Agree", roundtrip::<eolib::protocol::net::server::PlayersAgreeServerPacket>),
    ("server", "Players", "Net242", roundtrip::<eolib::protocol::net::server::PlayersNet242ServerPacket>),
    ("server", "Players", "Ping", roundtrip::<eolib::protocol::net::server::PlayersPingServerPacket>),
    ("server", "Players", "Pong", roundtrip::<eolib::protocol::net::server::PlayersPongServerPacket>),
    ("server", "Players", "Remove", roundtrip::<eolib::protocol::net::server::PlayersRemoveServerPacket>),
    ("server", "Quest", "Dialog", roundtrip::<eolib::protocol::net::server::QuestDialogServerPacket>),
    ("server", "Quest", "List", roundtrip::<eolib::protocol::net::server::QuestListServerPacket>),
    ("server", "Range", "Reply", roundtrip::<eolib::protocol::net::server::RangeReplyServerPacket>),
    ("server", "Recover", "List", roundtrip::<eolib::protocol::net::server::RecoverListServerPacket>),
    ("server", "Recover", "Player", roundtrip::<eolib::protocol::net::server::RecoverPlayerServerPacket>),
    ("server", "Recover", "Reply", roundtrip::<eolib::protocol::net::server::RecoverReplyServerPacket>),
    ("server", "Shop", "Open", roundtrip::<eolib::protocol::net::server::ShopOpenServerPacket>),
    ("server", "Sit", "Close", roundtrip::<eolib::protocol::net::server::SitCloseServerPacket>),
    ("server", "Sit", "Reply", roundtrip::<eolib::protocol::net::server::SitReplyServerPacket>),
    ("server", "Spell", "Reply", roundtrip::<eolib::protocol::net::server::SpellReplyServerPacket>),
    ("server", "Spell", "TargetOther", roundtrip::<eolib::protocol::net::server::SpellTargetOtherServerPacket>),
    ("server", "StatSkill", "Open", roundtrip::<eolib::protocol::net::server::StatSkillOpenServerPacket>),
    ("server", "Talk", "Announce", roundtrip::<eolib::protocol::net::server::TalkAnnounceServerPacket>),
    ("server", "Talk", "List", roundtrip::<eolib::protocol::net::server::TalkListServerPacket>),
    ("server", "Talk", "Msg", roundtrip::<eolib::protocol::net::server::TalkMsgServerPacket>),
    ("server", "Talk", "Open", roundtrip::<eolib::protocol::net::server::TalkOpenServerPacket>),
    ("server", "Talk", "Player", roundtrip::<eolib::protocol::net::server::TalkPlayerServerPacket>),
    ("server", "Talk", "Reply", roundtrip::<eolib::protocol::net::server::TalkReplyServerPacket>),
    ("server", "Talk", "Server", roundtrip::<eolib::protocol::net::server::TalkServerServerPacket>),
    ("server", "Talk", "Spec", roundtrip::<eolib::protocol::net::server::TalkSpecServerPacket>),
    ("server", "Trade", "Agree", roundtrip::<eolib::protocol::net::server::TradeAgreeServerPacket>),
    ("server", "Trade", "Close", roundtrip::<eolib::protocol::net::server::TradeCloseServerPacket>),
    ("server", "Trade", "Open", roundtrip::<eolib::protocol::net::server::TradeOpenServerPacket>),
    ("server", "Trade", "Reply", roundtrip::<eolib::protocol::net::server::TradeReplyServerPacket>),
    ("server", "Trade", "Request", roundtrip::<eolib::protocol::net::server::TradeRequestServerPacket>),
    ("server", "Trade", "Spec", roundtrip::<eolib::protocol::net::server::TradeSpecServerPacket>),
    ("server", "Trade", "Use", roundtrip::<eolib::protocol::net::server::TradeUseServerPacket>),
    ("server", "Walk", "Close", roundtrip::<eolib::protocol::net::server::WalkCloseServerPacket>),
    ("server", "Walk", "Open", roundtrip::<eolib::protocol::net::server::WalkOpenServerPacket>),
    ("server", "Walk", "Player", roundtrip::<eolib::protocol::net::server::WalkPlayerServerPacket>),
    ("server", "Walk", "Reply", roundtrip::<eolib::protocol::net::server::WalkReplyServerPacket>),
    ("server", "Warp", "Agree", roundtrip::<eolib::protocol::net::server::WarpAgreeServerPacket>),
    ("server", "Warp", "Request", roundtrip::<eolib::protocol::net::server::WarpRequestServerPacket>),
    ("server", "Welcome", "Reply", roundtrip::<eolib::protocol::net::server::WelcomeReplyServerPacket>),
];

/// Deserializes `data` as `T` and re-serializes it, returning the resulting bytes.
fn roundtrip<T: EoSerialize>(data: &[u8]) -> Result<Vec<u8>, String> {
    let reader = EoReader::new(Bytes::copy_from_slice(data));
    let packet = T::deserialize(&reader).map_err(|e| e.to_string())?;

    let mut writer = EoWriter::new();
    packet.serialize(&mut writer).map_err(|e| e.to_string())?;

    Ok(writer.to_byte_array().to_vec())
}

fn decode_base64(input: &str) -> Vec<u8> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|b| *b != b'\n' && *b != b'\r')
        .collect();

    for chunk in bytes.chunks(4) {
        let a = value(chunk[0]).expect("invalid base64 input");
        let b = value(chunk[1]).expect("invalid base64 input");
        output.push((a << 2) | (b >> 4));

        if chunk.len() > 2 && chunk[2] != b'=' {
            let c = value(chunk[2]).expect("invalid base64 input");
            output.push((b << 4) | (c >> 2));

            if chunk.len() > 3 && chunk[3] != b'=' {
                let d = value(chunk[3]).expect("invalid base64 input");
                output.push((c << 6) | d);
            }
        }
    }

    output
}

#[test]
fn captured_packets_roundtrip() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("eo-captured-packets");
    assert!(
        root.is_dir(),
        "captured packets are missing at {}. Initialize submodules with `git submodule update --init --recursive`.",
        root.display()
    );

    let mut tested = 0;
    let mut failures: Vec<String> = Vec::new();

    for source in ["client", "server"] {
        let dir = root.join(source);
        let mut paths: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension().and_then(|extension| extension.to_str()) == Some("json")
            })
            .collect();
        paths.sort();

        for path in paths {
            let text = fs::read_to_string(&path).unwrap();
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();

            let family = value["family"].as_str().unwrap();
            let action = value["action"].as_str().unwrap();
            let expected = decode_base64(value["expected"].as_str().unwrap_or(""));

            let Some((_, _, _, roundtrip)) = PACKETS
                .iter()
                .find(|(s, f, a, _)| *s == source && *f == family && *a == action)
            else {
                failures.push(format!(
                    "{}: no generated packet registered for {source}/{family}/{action}",
                    path.display()
                ));
                continue;
            };

            match roundtrip(&expected) {
                Ok(actual) if actual == expected => {}
                Ok(actual) => failures.push(format!(
                    "{}: round-trip mismatch\n    expected: {expected:02x?}\n    actual:   {actual:02x?}",
                    path.display()
                )),
                Err(error) => failures.push(format!("{}: {error}", path.display())),
            }

            tested += 1;
        }
    }

    assert!(tested > 0, "no captured packets were tested");
    assert!(
        failures.is_empty(),
        "{} of {tested} captured packets failed to round-trip:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
