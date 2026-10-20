// Two player play, the host simulates and streams its ticks while the helper's input flows back
pub mod dialog;
pub mod hash;
pub mod session;
pub mod wire;

pub use session::{Hand, Notice, Phase, Role, Session, Who};

// Banner text naming the winner of a friend match
pub fn wins_text(name: &str) -> String {
    format!("{name} WINS!")
}

// Banner text for a friend match that ended level
pub fn tie_text() -> &'static str {
    "TIE!"
}

#[cfg(test)]
mod tests {
    #[test]
    fn banner_names_the_winner() {
        assert_eq!(super::wins_text("MATE"), "MATE WINS!");
        assert_eq!(super::tie_text(), "TIE!");
    }
}
