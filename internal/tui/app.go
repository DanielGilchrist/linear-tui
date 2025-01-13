package tui

import (
	"fmt"

	"github.com/DanielGilchrist/linear-tui/internal/api"
	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
)

type Model struct {
	client       *api.Client
	teams        []api.Team
	teamsList    list.Model
	selectedTeam *api.Team
	err          error
}

func NewModel(client *api.Client) Model {
	return Model{
		client:    client,
		teamsList: list.New(nil, list.NewDefaultDelegate(), 0, 0),
	}
}

func (m Model) Init() tea.Cmd {
	return m.fetchTeams
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return m, tea.Quit
		case "enter":
			if i := m.teamsList.Index(); i != -1 && i < len(m.teams) {
				m.selectedTeam = &m.teams[i]
			}
		}

	case teamsMsg:
		m.teams = msg.teams
		items := make([]list.Item, len(m.teams))
		for i, team := range m.teams {
			items[i] = teamItem{team}
		}
		m.teamsList.SetItems(items)

	case errMsg:
		m.err = msg.err
	}

	var cmd tea.Cmd
	m.teamsList, cmd = m.teamsList.Update(msg)
	return m, cmd
}

func (m Model) View() string {
	if m.err != nil {
		return "Error: " + m.err.Error()
	}

	if m.selectedTeam != nil {
		return "Selected team: " + m.selectedTeam.Name
	}

	return "Teams\n" + m.teamsList.View()
}

type teamsMsg struct {
	teams []api.Team
}

type errMsg struct {
	err error
}

func (m Model) fetchTeams() tea.Msg {
	resp, err := m.client.GetTeamsWithIssues()
	if err != nil {
		return errMsg{err}
	}
	return teamsMsg{teams: resp.Data.Teams.Nodes}
}

type teamItem struct {
	team api.Team
}

func (i teamItem) Title() string {
	return i.team.Name
}

func (i teamItem) Description() string {
	return fmt.Sprintf("%d issues", i.team.IssueCount)
}

func (i teamItem) FilterValue() string {
	return i.team.Name
}
