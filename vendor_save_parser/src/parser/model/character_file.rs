use serde::{Deserialize, Serialize};

use crate::util::{ensure_eq, Result};

use super::{
    super::{Parser, Readable},
    CharacterBio, CharacterInfo, CharacterSkills, CharacterStash, FactionPack, Header, Inventory,
    LoreNotes, MarkerList, PlayStats, RespawnList, ShrineList, TeleportList, TriggerTokens,
    TutorialPages, UISettings, UID,
};

#[derive(Deserialize, Serialize)]
pub struct CharacterFile {
    hdr: Header,
    id: UID,
    info: CharacterInfo,
    bio: CharacterBio,
    inv: Inventory,
    stash: CharacterStash,
    respawns: RespawnList,
    teleports: TeleportList,
    markers: MarkerList,
    shrines: ShrineList,
    skills: CharacterSkills,
    notes: LoreNotes,
    factions: FactionPack,
    ui: UISettings,
    tutorials: TutorialPages,
    stats: PlayStats,
    tokens: TriggerTokens,
}

impl Readable for CharacterFile {
    fn read_from(reader: &mut dyn Parser) -> Result<Self>
    where
        Self: Sized,
    {
        ensure_eq(reader.read_int()?, 0x58434447, "start bytes 0")?;
        ensure_eq(reader.read_int()?, 2, "start bytes 1")?;
        let hdr = Header::read_from(reader)?;
        ensure_eq(reader.read_byte()?, 3, "start bytes 2")?;
        ensure_eq(reader.next_int()?, 0, "start bytes 3")?;
        ensure_eq(reader.read_int()?, 8, "version")?;
        let id = UID::read_from(reader)?;
        let info = CharacterInfo::read_from(reader)?;
        let bio = CharacterBio::read_from(reader)?;
        let inv = Inventory::read_from(reader)?;
        let stash = CharacterStash::read_from(reader)?;
        let respawns = RespawnList::read_from(reader)?;
        let teleports = TeleportList::read_from(reader)?;
        let markers = MarkerList::read_from(reader)?;
        let shrines = ShrineList::read_from(reader)?;
        let skills = CharacterSkills::read_from(reader)?;
        let notes = LoreNotes::read_from(reader)?;
        let factions = FactionPack::read_from(reader)?;
        let ui = UISettings::read_from(reader)?;
        let tutorials = TutorialPages::read_from(reader)?;
        let stats = PlayStats::read_from(reader)?;
        let tokens = TriggerTokens::read_from(reader)?;

        Ok(CharacterFile {
            hdr,
            id,
            info,
            bio,
            inv,
            stash,
            respawns,
            teleports,
            markers,
            shrines,
            skills,
            notes,
            factions,
            ui,
            tutorials,
            stats,
            tokens,
        })
    }
}
