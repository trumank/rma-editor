//! Enum string conversions between UE4 property values and Rust enum types.

use crate::objects::{
    ECaveEntrancePriority, ECaveEntranceType, EItemAdjustmentType, ERoomMirroringSupport,
};

/// Parse ECaveEntranceType from UE4 enum string
pub fn parse_entrance_type(s: Option<&str>) -> ECaveEntranceType {
    match s {
        Some(s) if s.ends_with("EntranceAndExit") => ECaveEntranceType::EntranceAndExit,
        Some(s) if s.ends_with("Entrance") => ECaveEntranceType::Entrance,
        Some(s) if s.ends_with("Exit") => ECaveEntranceType::Exit,
        Some(s) if s.ends_with("TreassureRoom") => ECaveEntranceType::TreassureRoom,
        _ => ECaveEntranceType::EntranceAndExit,
    }
}

/// Convert ECaveEntranceType to UE4 enum string
pub fn entrance_type_to_string(t: ECaveEntranceType) -> &'static str {
    match t {
        ECaveEntranceType::EntranceAndExit => "ECaveEntranceType::EntranceAndExit",
        ECaveEntranceType::Entrance => "ECaveEntranceType::Entrance",
        ECaveEntranceType::Exit => "ECaveEntranceType::Exit",
        ECaveEntranceType::TreassureRoom => "ECaveEntranceType::TreassureRoom",
    }
}

/// Parse ECaveEntrancePriority from UE4 enum string
pub fn parse_entrance_priority(s: Option<&str>) -> ECaveEntrancePriority {
    match s {
        Some(s) if s.ends_with("Primary") => ECaveEntrancePriority::Primary,
        Some(s) if s.ends_with("Secondary") => ECaveEntrancePriority::Secondary,
        _ => ECaveEntrancePriority::Primary,
    }
}

/// Convert ECaveEntrancePriority to UE4 enum string
pub fn entrance_priority_to_string(p: ECaveEntrancePriority) -> &'static str {
    match p {
        ECaveEntrancePriority::Primary => "ECaveEntrancePriority::Primary",
        ECaveEntrancePriority::Secondary => "ECaveEntrancePriority::Secondary",
    }
}

/// Parse EItemAdjustmentType from UE4 enum string
pub fn parse_adjustment_type(s: Option<&str>) -> EItemAdjustmentType {
    match s {
        Some(s) if s.ends_with("Cieling") => EItemAdjustmentType::Cieling,
        Some(s) if s.ends_with("Wall") => EItemAdjustmentType::Wall,
        Some(s) if s.ends_with("Floor") => EItemAdjustmentType::Floor,
        Some(s) if s.ends_with("None") => EItemAdjustmentType::None,
        _ => EItemAdjustmentType::None,
    }
}

/// Convert EItemAdjustmentType to UE4 enum string
pub fn adjustment_type_to_string(t: EItemAdjustmentType) -> &'static str {
    match t {
        EItemAdjustmentType::None => "EItemAdjustmentType::None",
        EItemAdjustmentType::Cieling => "EItemAdjustmentType::Cieling",
        EItemAdjustmentType::Wall => "EItemAdjustmentType::Wall",
        EItemAdjustmentType::Floor => "EItemAdjustmentType::Floor",
    }
}

/// Parse ERoomMirroringSupport from UE4 enum string
pub fn parse_mirroring_support(s: Option<&str>) -> ERoomMirroringSupport {
    match s {
        Some(s) if s.ends_with("NotAllowed") => ERoomMirroringSupport::NotAllowed,
        Some(s) if s.ends_with("MirrorAroundX") => ERoomMirroringSupport::MirrorAroundX,
        Some(s) if s.ends_with("MirrorAroundY") => ERoomMirroringSupport::MirrorAroundY,
        Some(s) if s.ends_with("MirrorBoth") => ERoomMirroringSupport::MirrorBoth,
        _ => ERoomMirroringSupport::NotAllowed,
    }
}

/// Convert ERoomMirroringSupport to UE4 enum string
pub fn mirroring_support_to_string(m: ERoomMirroringSupport) -> &'static str {
    match m {
        ERoomMirroringSupport::NotAllowed => "ERoomMirroringSupport::NotAllowed",
        ERoomMirroringSupport::MirrorAroundX => "ERoomMirroringSupport::MirrorAroundX",
        ERoomMirroringSupport::MirrorAroundY => "ERoomMirroringSupport::MirrorAroundY",
        ERoomMirroringSupport::MirrorBoth => "ERoomMirroringSupport::MirrorBoth",
    }
}

/// Extract the class name from an ObjectRef path (e.g., "/Script/FSD.FloodFillBox" -> "FloodFillBox")
pub fn extract_class_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}
