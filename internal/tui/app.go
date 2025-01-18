package tui

import (
	"fmt"
	"strconv"

	"github.com/DanielGilchrist/linear-tui/internal/api"
	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

type appState int

const (
	stateTeamsList appState = iota
	stateIssuesList
	stateIssueDetail
)

type Model struct {
	// Components
	teamsList  list.Model
	issuesList list.Model
	spinner    spinner.Model
	client     *api.Client

	// State
	state         appState
	selectedTeam  *api.Team
	selectedIssue *api.Issue
	loadingTeams  bool
	loadingIssues bool

	// Layout
	width  int
	height int
	err    error
}

func New(client *api.Client) Model {
	teamsList := newList("Teams")
	issuesList := newList("Issues")
	sp := newSpinner()

	return Model{
		teamsList:    teamsList,
		issuesList:   issuesList,
		spinner:      sp,
		client:       client,
		state:        stateTeamsList,
		loadingTeams: true,
	}
}

func newList(title string) list.Model {
	list := list.New([]list.Item{}, list.NewDefaultDelegate(), 0, 0)
	list.Title = title
	list.SetShowHelp(false)

	return list
}

func newSpinner() spinner.Model {
	sp := spinner.New()
	sp.Spinner = spinner.Dot
	sp.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("69"))

	return sp
}

func (model Model) Init() tea.Cmd {
	return tea.Batch(
		model.spinner.Tick,
		model.loadTeams,
	)
}

func (model Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		model.height = msg.Height
		model.width = msg.Width

		switch model.state {
		case stateTeamsList, stateIssuesList:
			panelWidth := (model.width - 2) / 2
			model.teamsList.SetSize(panelWidth, model.height)
			model.issuesList.SetSize(panelWidth, model.height)
		case stateIssueDetail:
			model.teamsList.SetSize(model.width, model.height)
		}

		return model, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return model, tea.Quit
		case "esc", "backspace":
			switch model.state {
			case stateIssueDetail:
				model.state = stateIssuesList
			case stateIssuesList:
				model.state = stateTeamsList
			}
		case "enter":
			switch model.state {
			case stateTeamsList:
				if teamItem, ok := model.teamsList.SelectedItem().(teamItem); ok {
					team := &teamItem.team
					model.selectedTeam = team
					model.loadingIssues = true
					model.state = stateIssuesList

					return model, tea.Batch(model.spinner.Tick, model.loadIssuesCmd(team.ID))
				}
			case stateIssuesList:
				if issueItem, ok := model.issuesList.SelectedItem().(issueItem); ok {
					issue := &issueItem.issue
					model.selectedIssue = issue
					// TODO: Set loading individual issue
					model.state = stateIssueDetail
				}
			}
		}

		switch model.state {
		case stateTeamsList:
			model.teamsList, cmd = model.teamsList.Update(msg)
			cmds = append(cmds, cmd)
		case stateIssuesList:
			model.issuesList, cmd = model.issuesList.Update(msg)
			cmds = append(cmds, cmd)
		}

		return model, tea.Batch(cmds...)

	case teamsLoadedMsg:
		model.loadingTeams = false
		items := make([]list.Item, len(msg.teams))

		for i, team := range msg.teams {
			items[i] = teamItem{team}
		}

		model.teamsList.SetItems(items)
		return model, nil

	case issuesLoadedMsg:
		model.loadingIssues = false
		items := make([]list.Item, len(msg.issues))

		for i, issue := range msg.issues {
			items[i] = issueItem{issue}
		}

		model.issuesList.SetItems(items)
		return model, nil

	case errorMsg:
		model.loadingTeams = false
		model.loadingIssues = false
		model.err = msg.err

		return model, nil
	}

	if model.loadingTeams || model.loadingIssues {
		model.spinner, cmd = model.spinner.Update(msg)
		cmds = append(cmds, cmd)
	}

	switch model.state {
	case stateTeamsList, stateIssuesList:
		model.teamsList, cmd = model.teamsList.Update(msg)
		cmds := append(cmds, cmd)

		if model.state == stateIssuesList {
			model.issuesList, cmd = model.issuesList.Update(msg)
			cmds = append(cmds, cmd)
		}
	}

	return model, tea.Batch(cmds...)
}

func (model Model) View() string {
	if model.err != nil {
		return fmt.Sprintf("Error: %v", model.err)
	}

	switch model.state {
	case stateTeamsList, stateIssuesList:
		var issuesPanel string

		if model.state == stateIssuesList {
			if model.loadingIssues {
				issuesPanel = model.renderSpinner()
			} else {
				issuesPanel = model.issuesList.View()
			}
		} else {
			issuesPanel = "Select a team to view issues"
		}

		panels := []string{
			model.teamsList.View(),
			issuesPanel,
		}

		return lipgloss.JoinHorizontal(lipgloss.Top, panels...)
	case stateIssueDetail:
		if model.selectedIssue == nil {
			return "No issue selected"
		}

		return model.renderIssueDetail()
	}

	panic("Invalid state!")
}

func (model Model) renderSpinner() string {
	return lipgloss.NewStyle().
		Width(model.width).
		Height(model.height).
		Render(model.spinner.View())
}

func (model Model) renderIssueDetail() string {
	return "TODO: Implement rendering issue"
}

type teamsLoadedMsg struct{ teams []api.Team }
type issuesLoadedMsg struct{ issues []api.Issue }
type errorMsg struct{ err error }

func (model Model) loadTeams() tea.Msg {
	teams, err := model.client.GetTeams()
	if err != nil {
		return errorMsg{err}
	}

	return teamsLoadedMsg{teams.Data.Teams.Nodes}
}

func (model Model) loadIssuesCmd(teamId string) tea.Cmd {
	return func() tea.Msg {
		issues, err := model.client.GetTeamIssues(teamId)
		if err != nil {
			return errorMsg{err}
		}

		return issuesLoadedMsg{issues.Data.Team.Issues.Nodes}
	}
}

type teamItem struct {
	team api.Team
}

func (t teamItem) Title() string       { return t.team.Name }
func (t teamItem) Description() string { return fmt.Sprintf("%d issues", t.team.IssueCount) }
func (t teamItem) FilterValue() string { return t.team.Name }

type issueItem struct {
	issue api.Issue
}

func (i issueItem) Title() string       { return i.issue.Title }
func (i issueItem) Description() string { return i.issue.Description }
func (i issueItem) FilterValue() string {
	return strconv.FormatFloat(float64(i.issue.SortOrder), 'f', -1, 32)
}
