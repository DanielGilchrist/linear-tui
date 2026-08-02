use std::num::NonZeroUsize;

use super::feed::FeedKey;
use super::focus::{Cursor, Direction};
use crate::api::{IssueFilter, StateType, Team, TeamId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamMode {
    Active,
    Triage,
    Backlog,
    All,
}

impl TeamMode {
    pub fn label(self) -> &'static str {
        match self {
            TeamMode::Active => "active",
            TeamMode::Triage => "triage",
            TeamMode::Backlog => "backlog",
            TeamMode::All => "all",
        }
    }

    pub fn filter(self, team: &TeamId) -> IssueFilter {
        let (state_types_in, state_types_nin) = match self {
            TeamMode::Active => (vec![StateType::Unstarted, StateType::Started], Vec::new()),
            TeamMode::Triage => (vec![StateType::Triage], Vec::new()),
            TeamMode::Backlog => (vec![StateType::Backlog], Vec::new()),
            TeamMode::All => (Vec::new(), vec![StateType::Completed, StateType::Cancelled]),
        };

        IssueFilter {
            state_types_in,
            state_types_not_in: state_types_nin,
            team: Some(team.clone()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct TeamModes(Vec<TeamMode>);

impl TeamModes {
    pub fn for_team(team: &Team) -> Self {
        let mut modes = vec![TeamMode::Active];

        if team.triage_enabled {
            modes.push(TeamMode::Triage);
        }

        modes.extend([TeamMode::Backlog, TeamMode::All]);

        TeamModes(modes)
    }

    pub fn as_slice(&self) -> &[TeamMode] {
        &self.0
    }

    pub fn len(&self) -> NonZeroUsize {
        NonZeroUsize::MIN.saturating_add(self.0.len() - 1)
    }

    pub fn at(&self, cursor: Cursor) -> TeamMode {
        self.0[cursor.index().min(self.0.len() - 1)]
    }
}

#[derive(Debug, Clone)]
pub struct TeamSurface {
    team: TeamId,
    name: String,
    modes: TeamModes,
    at: Cursor,
}

impl TeamSurface {
    pub fn new(team: &Team) -> Self {
        Self {
            team: team.id.clone(),
            name: team.name.clone(),
            modes: TeamModes::for_team(team),
            at: Cursor::first(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mode(&self) -> TeamMode {
        self.modes.at(self.at)
    }

    pub fn filter(&self) -> IssueFilter {
        self.mode().filter(&self.team)
    }

    pub fn key(&self) -> FeedKey {
        FeedKey::Issues(self.filter())
    }

    pub fn cycle(&mut self, direction: Direction) {
        self.at = self.at.stepped(self.modes.len().get(), direction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team(triage_enabled: bool) -> Team {
        Team {
            id: TeamId::from_raw("t_pizza"),
            name: "Pizza".into(),
            key: "DAN2".into(),
            triage_enabled,
        }
    }

    #[test]
    fn every_mode_maps_to_a_distinct_team_scoped_filter() {
        let team = team(true);
        let modes = TeamModes::for_team(&team);

        let keys: Vec<FeedKey> = (0..modes.len().get())
            .map(|index| {
                let mode = modes.at(Cursor::new(index, modes.len().get()).expect("in range"));

                FeedKey::Issues(mode.filter(&team.id))
            })
            .collect();

        for key in &keys {
            match key {
                FeedKey::Issues(filter) => {
                    assert_eq!(filter.team.as_ref(), Some(&team.id));
                }
                other => panic!("expected a team-scoped issue feed, got {other:?}"),
            }
        }

        let unique: std::collections::HashSet<&FeedKey> = keys.iter().collect();

        assert_eq!(unique.len(), keys.len(), "each mode caches separately");
    }

    #[test]
    fn a_team_without_a_queue_has_no_triage_mode() {
        let mut surface = TeamSurface::new(&team(false));
        let mut seen = Vec::new();

        for _ in 0..8 {
            seen.push(surface.mode());
            surface.cycle(Direction::Next);
        }

        assert!(
            !seen.contains(&TeamMode::Triage),
            "the value does not exist to cycle onto, rather than being skipped"
        );

        let enabled = TeamSurface::new(&team(true));
        let mut enabled_seen = Vec::new();
        let mut enabled = enabled;

        for _ in 0..8 {
            enabled_seen.push(enabled.mode());
            enabled.cycle(Direction::Next);
        }

        assert!(enabled_seen.contains(&TeamMode::Triage));
    }

    #[test]
    fn triage_mode_filters_on_the_triage_state_type() {
        let team = team(true);
        let filter = TeamMode::Triage.filter(&team.id);

        assert_eq!(filter.state_types_in, vec![StateType::Triage]);
        assert!(filter.state_types_not_in.is_empty());
        assert_eq!(filter.team.as_ref(), Some(&team.id));
    }

    #[test]
    fn triage_is_one_step_from_the_default() {
        let mut surface = TeamSurface::new(&team(true));
        surface.cycle(Direction::Next);

        assert_eq!(surface.mode(), TeamMode::Triage);
    }

    #[test]
    fn cycling_modes_wraps_and_never_leaves_the_list() {
        let mut surface = TeamSurface::new(&team(false));
        let seen: Vec<TeamMode> = (0..4)
            .map(|_| {
                let mode = surface.mode();
                surface.cycle(Direction::Next);

                mode
            })
            .collect();

        assert_eq!(
            seen,
            vec![
                TeamMode::Active,
                TeamMode::Backlog,
                TeamMode::All,
                TeamMode::Active
            ]
        );
    }

    #[test]
    fn a_team_surface_opens_on_the_browser_not_a_narrow_mode() {
        assert_eq!(TeamSurface::new(&team(false)).mode(), TeamMode::Active);
        assert_eq!(
            TeamSurface::new(&team(true)).mode(),
            TeamMode::Active,
            "a triage queue must not change where Enter lands"
        );
    }
}
