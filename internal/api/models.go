package api

type Team struct {
	ID         string `json:"id"`
	Name       string `json:"name"`
	Issues     Issues `json:"issues"`
	IssueCount int    `json:"issueCount"`
}

type Issue struct {
	ID          string  `json:"id"`
	Title       string  `json:"title"`
	Description string  `json:"description"`
	SortOrder   float32 `json:"sortOrder"`
}

type Teams struct {
	Nodes []Team `json:"nodes"`
}

type Issues struct {
	Nodes []Issue `json:"nodes"`
}

type TeamsResponse struct {
	Data struct {
		Teams Teams `json:"teams"`
	} `json:"data"`
}
