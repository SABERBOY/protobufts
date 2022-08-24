use crate::proto::{
    error::ProtoError,
    proto_scope::{root_scope::RootScope, ProtoScope},
};

use super::ast::Folder;

pub(super) fn compile_decode(
    root: &RootScope,
    message_folder: &mut Folder,
    message_scope: &ProtoScope,
) -> Result<(), ProtoError> {
    let file = super::ast::File::new("decode".into());

    message_folder.entries.push(file.into());
    Ok(())
}
