use crate::api::FileFlags;
use lock_api::RawMutex;
use undefined_vfs::mount::Location;

pub struct Directory<M> {
    location: Location<M>,
    flags: FileFlags,
    position: u64,
}

impl<M: RawMutex> Directory<M> {
    pub(crate) fn new(location: Location<M>, flags: FileFlags) -> Self {
        Directory {
            location,
            flags,
            position: 0,
        }
    }

    pub fn location(&self) -> &Location<M> {
        &self.location
    }

    pub fn get_flags(&self) -> FileFlags {
        self.flags
    }

    // TODO: set_flags(&mut self, flags: FileFlags)

    pub fn get_position(&self) -> u64 {
        self.position
    }

    pub fn set_position(&mut self, position: u64) {
        self.position = position;
    }
}
