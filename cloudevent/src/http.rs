pub const CE_ID_HEADER: &str = "ce-id";
pub const CE_TYPE_HEADER: &str = "ce-type";
pub const CE_SOURCE_HEADER: &str = "ce-source";
pub const CE_SPECVERSION_HEADER: &str = "ce-specversion";

pub const CE_SUBJECT_HEADER: &str = "ce-subject";
pub const CE_TIME_HEADER: &str = "ce-time";

pub enum Encoding {
    BINARY,
    STRUCTURED,
    BATCH,
}
