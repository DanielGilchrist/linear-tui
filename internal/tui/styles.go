package tui

import "github.com/charmbracelet/lipgloss"

var (
	subtle    = lipgloss.AdaptiveColor{Light: "#D9DCCF", Dark: "#383838"}
	highlight = lipgloss.AdaptiveColor{Light: "#874BFD", Dark: "#7D56F4"}
	special   = lipgloss.AdaptiveColor{Light: "#43BF6D", Dark: "#73F59F"}

	normalBorder = lipgloss.Border{
		Top:         "─",
		Bottom:      "─",
		Left:        "│",
		Right:       "│",
		TopLeft:     "╭",
		TopRight:    "╮",
		BottomLeft:  "╰",
		BottomRight: "╯",
	}

	baseStyle = lipgloss.NewStyle().
			BorderStyle(normalBorder).
			BorderForeground(subtle)

	sidebarStyle = baseStyle.
			BorderRight(true).
			BorderLeft(true).
			BorderTop(true).
			BorderBottom(true).
			Padding(0, 1)

	mainStyle = baseStyle.
			BorderRight(true).
			BorderLeft(false).
			BorderTop(true).
			BorderBottom(true).
			Padding(0, 1)

	footerStyle = lipgloss.NewStyle().
			BorderStyle(normalBorder).
			BorderTop(true).
			BorderLeft(true).
			BorderRight(true).
			BorderBottom(true).
			Padding(0, 1).
			Align(lipgloss.Center).
			Foreground(lipgloss.AdaptiveColor{Light: "#666666", Dark: "#AAAAAA"})

	titleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(highlight).
			MarginLeft(2)

	selectedStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(special)
)
