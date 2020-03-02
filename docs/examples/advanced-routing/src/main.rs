mod router;
mod service;

use router::RootRouter;
use via::prelude::*;

fn main() -> Result<(), Error> {
    let mut app = App::new();

    app.mount(RootRouter);
    app.listen()
}
