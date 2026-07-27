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
	issuesList   list.Model
	teamsList    list.Model
	selectedTeam *api.Team
	width        int
	height       int
	err          error
}

func NewModel(client *api.Client) Model {
	teamsDelegate := list.NewDefaultDelegate()
	teamsDelegate.ShortHelpFunc = func() []key.Binding { return nil }
	teamsDelegate.FullHelpFunc = func() [][]key.Binding { return nil }

	teamsList := list.New(nil, teamsDelegate, 0, 0)
	teamsList.Title = "Teams"
	teamsList.SetShowHelp(false)
	teamsList.SetShowStatusBar(false)
	teamsList.SetFilteringEnabled(true)

	issuesDelegate := list.NewDefaultDelegate()
	issuesDelegate.ShortHelpFunc = func() []key.Binding { return nil }
	issuesDelegate.FullHelpFunc = func() [][]key.Binding { return nil }

	issuesList := list.New(nil, issuesDelegate, 0, 0)
	issuesList.Title = "Issues"
	issuesList.SetShowHelp(false)
	issuesList.SetShowStatusBar(false)
	issuesList.SetFilteringEnabled(true)

	return Model{
		client:     client,
		issuesList: issuesList,
		teamsList:  teamsList,
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

		headerHeight := 1
		footerHeight := 3
		availableHeight := m.height - headerHeight - footerHeight

		m.teamsList.SetSize(m.sidebarWidth(), m.contentHeight())
		m.issuesList.SetSize(m.mainWidth(), availableHeight)

	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return m, tea.Quit
		case "enter":
			if i := m.teamsList.Index(); i != -1 && i < len(m.teams) {
				m.selectedTeam = &m.teams[i]

				issueItems := make([]list.Item, len(m.selectedTeam.Issues.Nodes))
				for i, issue := range m.selectedTeam.Issues.Nodes {
					issueItems[i] = issueItem{issue: issue}
				}

				m.issuesList.SetItems(issueItems)
				m.issuesList.Title = m.selectedTeam.Name
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

	m.issuesList, cmd = m.issuesList.Update(msg)
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
	var content string
	if m.selectedTeam == nil {
		content = "No team selected"
	} else {
		content = m.issuesList.View()
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

type issueItem struct {
	issue api.Issue
}

func (i issueItem) Title() string {
	return i.issue.Title
}

func (i issueItem) Description() string {
	return i.issue.Description
}

func (i issueItem) FilterValue() string {
	return i.issue.Title
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
