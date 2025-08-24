use ratatui::{layout::Rect, Frame};

pub mod issue_detail;
pub mod issues_list;
pub mod styled_list;
pub mod teams_list;

pub trait Renderable {
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

pub use issue_detail::IssueDetail;
pub use issues_list::IssuesList;
pub use teams_list::TeamsList;
