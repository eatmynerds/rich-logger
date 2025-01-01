use rich_logger::json;
use serde::Serialize;

#[derive(Default, Serialize)]
struct Foo {
    x: u8,
    foo: String,
}

fn main() {
    json(&Foo { x: 42, foo: "sdfsdfkjsdflkjsdflkjsdflkjsdflkjsdlkfjsldkfjsldkfjsdfjksdlfkjsdlkfjsdlkfjsdkfjsdflkjsdlfkjsdlfkjsdlfkjsdflkjsdflkjsdflkjsdfkljsdflkjsdlkfjsdflkjsdlfkjsdlfkjsdlfkjsdlfkjsdlkfjsldkfjsldkfjsldkfjsldkfjasldfjsladkjflasdjflkasjdflsjdflksjdfljsdfljsadlkfjsldkfjsdlkfjsldfkjsldkfjsldkfjsdlfkjsdlkfjsdlfkjsdlkfjsdlfkjsdlfkj".to_string() }, log::Level::Info);

}
