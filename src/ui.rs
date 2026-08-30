pub fn info(quiet: bool, message: impl AsRef<str>) {
    if quiet {
        return;
    }
    println!("{}", message.as_ref());
}

pub fn warn(quiet: bool, message: impl AsRef<str>) {
    if quiet {
        return;
    }
    eprintln!("warning: {}", message.as_ref());
}

pub fn disclaimer(quiet: bool) {
    if quiet {
        return;
    }

    warn(
        false,
        "linkmap is an experimental hobby project and is still under review. Use at your own risk.",
    );
    warn(
        false,
        "If you find issues, please contact Ashley via LinkedIn: https://www.linkedin.com/in/ashley-immanuel-81609731b/",
    );
}
