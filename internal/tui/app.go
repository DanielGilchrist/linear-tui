package tui

import (
	"fmt"
	"strings"

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
	client     *api.Client

	// State
	state         appState
	selectedTeam  *api.Team
	selectedIssue *api.Issue

	// Layout
	width  int
	height int
	err    error
}

func New(client *api.Client) Model {
	teamsList := newList("Teams")
	issuesList := newList("Issues")

	return Model{
		teamsList:  teamsList,
		issuesList: issuesList,
		client:     client,
		state:      stateTeamsList,
	}
}

func newList(title string) list.Model {
	delegate := list.NewDefaultDelegate()

	list := list.New([]list.Item{}, delegate, 0, 0)
	list.Title = title
	list.SetShowHelp(false)
	list.SetShowTitle(true)
	list.SetSpinner(spinner.Dot)

	return list
}

func (model Model) Init() tea.Cmd {
	return tea.Batch(
		model.teamsList.StartSpinner(),
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
			panelWidth := model.width
			model.teamsList.SetSize(panelWidth/3, model.height)
			model.issuesList.SetSize(panelWidth-20, model.height)
		case stateIssueDetail:
			model.teamsList.SetSize(model.width, model.height)
		}

		return model, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c":
			return model, tea.Quit
		case "shift+tab":
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
					model.state = stateIssuesList

					return model, tea.Batch(model.issuesList.StartSpinner(), model.loadIssuesCmd(team.ID))
				}
			case stateIssuesList:
				if teamIssueItem, ok := model.issuesList.SelectedItem().(teamIssueItem); ok {
					model.state = stateIssueDetail

					return model, model.loadIssueCmd(teamIssueItem.issue.ID)
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
		model.teamsList.StopSpinner()
		items := make([]list.Item, len(msg.teams))

		for i, team := range msg.teams {
			items[i] = teamItem{team}
		}

		model.teamsList.SetItems(items)
		return model, nil

	case issuesLoadedMsg:
		model.issuesList.StopSpinner()
		items := make([]list.Item, len(msg.issues))

		for i, issue := range msg.issues {
			items[i] = teamIssueItem{issue: issue, descWidth: model.width}
		}

		model.issuesList.SetItems(items)
		return model, nil

	case issueLoadedMsg:
		model.selectedIssue = &msg.issue
		return model, nil

	case errorMsg:
		model.teamsList.StopSpinner()
		model.issuesList.StopSpinner()
		model.err = msg.err

		return model, nil
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
}

func (model Model) View() string {
	if model.err != nil {
		return fmt.Sprintf("Error: %v", model.err)
	}

	switch model.state {
	case stateTeamsList, stateIssuesList:
		var issuesPanel string

		if model.state == stateIssuesList {
			issuesPanel = model.issuesList.View()
		} else {
			issuesPanel = "Select a team to view issues"
		}

		panels := []string{
			model.teamsList.View(),
			issuesPanel,
		}

		return lipgloss.JoinHorizontal(lipgloss.Top, panels...)
	case stateIssueDetail:
		return model.renderIssueDetail()
	}

	panic("Invalid state!")
}

func (model Model) renderIssueDetail() string {
	issue := model.selectedIssue

	if issue == nil {
		return "No issue selected"
	}

	var comments strings.Builder

	for _, comment := range issue.Comments.Nodes {
		comments.WriteString(fmt.Sprintf("\tBody:%s\n", comment.Body))
	}

	return fmt.Sprintf(
		"Title: %s\nDescription:%s\nComments:%s",
		issue.Title,
		issue.Description,
		comments.String(),
	)
}

type teamsLoadedMsg struct{ teams []api.Team }
type issuesLoadedMsg struct{ issues []api.TeamIssue }
type issueLoadedMsg struct{ issue api.Issue }
type errorMsg struct{ err error }

func (model Model) loadTeams() tea.Msg {
	teams, err := model.client.GetTeams()
	if err != nil {
		return errorMsg{err}
	}

	return teamsLoadedMsg{teams.Data.Teams.Nodes}
}

func (model Model) loadIssueCmd(issueId string) tea.Cmd {
	return func() tea.Msg {
		issue, err := model.client.GetIssue(issueId)
		if err != nil {
			return errorMsg{err}
		}

		return issueLoadedMsg{issue.Data.Issue}
	}
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

func (i teamItem) Title() string       { return i.team.Name }
func (i teamItem) Description() string { return fmt.Sprintf("%d issues", i.team.IssueCount) }
func (i teamItem) FilterValue() string { return i.team.Name }

type teamIssueItem struct {
	issue     api.TeamIssue
	descWidth int
}

func (i teamIssueItem) Title() string { return i.issue.Title }

func (i teamIssueItem) Description() string {
	limit := i.descWidth - 3
	description := i.issue.Description

	if len(description) == 0 {
		return "No description."
	}

	if len(description) < limit {
		return description
	}

	return description[:limit-3] + "..."
}

func (i teamIssueItem) FilterValue() string { return i.issue.Title }
