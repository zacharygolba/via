use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

pub struct Shared<App>(Arc<App>);

impl<App> Shared<App> {
    pub(super) fn new(value: App) -> Self {
        Self(Arc::new(value))
    }
}

impl<App> AsRef<App> for Shared<App> {
    #[inline]
    fn as_ref(&self) -> &App {
        &self.0
    }
}

impl<App> Clone for Shared<App> {
    #[inline]
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<App> Debug for Shared<App> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Shared").finish()
    }
}

impl<App> Deref for Shared<App> {
    type Target = App;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
