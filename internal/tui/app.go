package tui

import (
	"fmt"
	"strings"

	"github.com/DanielGilchrist/linear-tui/internal/api"
	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

const widthOffset = 4

type Model struct {
	teams        []api.Team
	client       *api.Client
	teamsList    list.Model
	selectedTeam *api.Team
	width        int
	height       int
	err          error
}

func NewModel(client *api.Client) Model {
	delegate := list.NewDefaultDelegate()

	delegate.ShortHelpFunc = func() []key.Binding { return nil }
	delegate.FullHelpFunc = func() [][]key.Binding { return nil }

	teamsList := list.New(nil, delegate, 0, 0)
	teamsList.Title = "Teams"
	teamsList.SetShowHelp(false)
	teamsList.SetShowStatusBar(false)
	teamsList.SetFilteringEnabled(true)

	return Model{
		client:    client,
		teamsList: teamsList,
	}
}

func (m Model) Init() tea.Cmd {
	return m.fetchTeams
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.teamsList.SetSize(m.sidebarWidth(), m.contentHeight())

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

	m.teamsList, cmd = m.teamsList.Update(msg)
	return m, cmd
}

func (m Model) View() string {
	if m.width == 0 {
		return "Loading..."
	}

	if m.err != nil {
		return "Error: " + m.err.Error()
	}

	sidebar := m.sidebarView()
	main := m.mainView()
	footer := m.footerView()

	top := lipgloss.JoinHorizontal(lipgloss.Top, sidebar, main)
	return lipgloss.JoinVertical(lipgloss.Top, top, footer)
}

func (m Model) sidebarView() string {
	return sidebarStyle.
		Width(m.sidebarWidth()).
		Height(m.contentHeight()).
		Render(m.teamsList.View())
}

func (m Model) mainView() string {
	content := "No team selected"
	if m.selectedTeam != nil {
		content = fmt.Sprintf(
			"%s\n\nIssues: %d",
			titleStyle.Render(m.selectedTeam.Name),
			m.selectedTeam.IssueCount,
		)
	}

	return mainStyle.
		Width(m.mainWidth()).
		Height(m.contentHeight()).
		Render(content)
}

func (m Model) footerView() string {
	keys := []string{
		"↑/↓: Navigate",
		"enter: Select",
		"q: Quit",
	}
	helpText := strings.Join(keys, " | ")
	return footerStyle.
		Width(m.width - widthOffset).
		Render(helpText)
}

func (m Model) sidebarWidth() int {
	return 30
}

func (m Model) mainWidth() int {
	return m.width - m.sidebarWidth() - widthOffset
}

func (m Model) contentHeight() int {
	return m.height - 10
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
