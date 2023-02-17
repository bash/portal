fn main() {
    #[cfg(windows)]
    embed_resource::compile("build/windows/icon.rc");
}
