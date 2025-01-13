package main

import (
	"fmt"
	"os"

	"github.com/DanielGilchrist/linear-tui/internal/api"
	"github.com/DanielGilchrist/linear-tui/internal/tui"
	tea "github.com/charmbracelet/bubbletea"
)

func main() {
	// TODO: Retrieve from user and store it somewhere safe on their machine
	apiToken := os.Getenv("LINEAR_API_TOKEN")

	if apiToken == "" {
		fmt.Println("Please set LINEAR_API_TOKEN environment variable")
		os.Exit(1)
	}

	client := api.NewClient(apiToken)
	model := tui.NewModel(client)
	program := tea.NewProgram(model, tea.WithAltScreen())

	if _, err := program.Run(); err != nil {
		fmt.Println("Error running program:", err)
		os.Exit(1)
	}
}
