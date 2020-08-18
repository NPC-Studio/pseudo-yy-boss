use thiserror::Error;

#[derive(Debug, Error, serde::Serialize, serde::Deserialize)]
pub enum FolderGraphError {
    #[error("path {} was not found", .0)]
    PathNotFound(String),

    #[error("folder already existed at that location")]
    FolderAlreadyPresent,

    #[error("file already existed at that location")]
    FileAlreadyPresent,

    #[error("foldergraph is out of sync with internal Yyp -- yypboss is in undefined state")]
    InternalError,

    #[error("couldn't remove folder, given file")]
    BadRemove,
}
