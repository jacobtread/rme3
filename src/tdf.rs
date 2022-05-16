use tokio::io::AsyncWrite;

#[repr(u8)]
enum TdfType {
    VARINT = 0x0,
    STRING = 0x1,
}

trait Tdf: Send + Sync {
    fn label(&self) -> String;
    fn tdf_type() -> TdfType;
    fn write<W: AsyncWrite>(&self, o: &mut W);
}

struct VarIntTdf(String, u32);

impl Tdf for VarIntTdf {
    fn label(&self) -> String {
        return self.0.clone();
    }

    fn tdf_type() -> TdfType {
        return TdfType::VARINT;
    }

    fn write<W: AsyncWrite>(&self, o: &mut W) {
        todo!()
    }
}

struct TdfBuilder {
    values: Vec<dyn Tdf<>>,
}

impl TdfBuilder {
    fn create() -> Self {
        return TdfBuilder {
            values: Vec::new()
        };
    }

    fn number(&mut self, label: &str, value: u32) -> &Self {
        self.values.push(VarIntTdf(String::from(label), value));
        return self;
    }
}

fn test() {
    TdfBuilder::create()
        .number("", 12);
}