fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../doc/logo.ico");
        res.compile().unwrap();
    }
}
