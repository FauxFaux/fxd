error_chain!{
    foreign_links {
        Io(::std::io::Error);
        PIE(::std::num::ParseIntError);
    }
}
