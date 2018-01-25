extern crate puddle;

extern crate iron;
extern crate persistent;

use std::fs::File;

use iron::prelude::*;
use iron::headers::ContentType;
use iron::status;
use iron::typemap::Key;

use persistent::State;

use puddle::api::Session;
use puddle::arch::Architecture;


struct SessionHolder;

impl Key for SessionHolder { type Value = puddle::api::Session; }


fn do_something(_: &mut Session) {}


fn serve(req: &mut Request) -> IronResult<Response> {
    let lock = req.get::<State<SessionHolder>>().unwrap();
    let mut session = lock.write().unwrap();

    do_something(&mut session);

    Ok(Response::with((status::Ok, format!("Hits: {}", 0))))
}


// fn variant1(_: &mut Request) -> IronResult<Response> {
//     let json_modifier = ContentType::json().0;
//     Ok(Response::with((json_modifier, status::Ok, "{}")))
// }


fn main() {
    let path = "../tests/arches/arch01.json";
    let reader = File::open(path).expect("file not found");
    let session = Session {
        arch: Architecture::from_reader(reader)
    };

    let mut chain = Chain::new(serve);
    chain.link(State::<SessionHolder>::both(session));

    let address = "localhost:3000";
    println!("Listening on http://{}", address);
    Iron::new(chain).http(address).unwrap();
}
