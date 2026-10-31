#[macro_export]
macro_rules! awrite {
    ($f:ident <- $s:literal $($args:tt)*)=> {
        $f.write_all(format!($s $($args)*).as_bytes()).await
    };
}
